//! Sound editor sidecars. A bounded temporary syntax tree is lowered into named
//! events, sample resources, library selections and material/environment enums.
//! No engine/editor runtime behavior is inferred from an unrecognized command.
use crate::model::audio::*;
use crate::{model::io::FormatSupport, openspace::codec::FormatCodec};
use anyhow::{Context, Result, bail, ensure};

#[derive(Default)]
struct Block {
    header: String,
    commands: Vec<(String, String)>,
    children: Vec<Block>,
}
fn command(line: &str) -> Result<(String, String)> {
    let start = line
        .find('(')
        .context("sound-script command has no argument list")?;
    ensure!(line.ends_with(')'), "unterminated sound-script command");
    let name = line[..start].split('[').next().unwrap().to_owned();
    Ok((name, line[start + 1..line.len() - 1].trim().to_owned()))
}
fn parse(text: &str) -> Result<Block> {
    ensure!(
        text.len() <= 32 * 1024 * 1024,
        "sound definitions exceed size limit"
    );
    let mut stack = vec![Block::default()];
    for (line_number, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        if let Some(header) = line.strip_prefix('{') {
            ensure!(stack.len() < 64, "sound-script nesting limit");
            stack.push(Block {
                header: header.to_owned(),
                ..Default::default()
            });
        } else if line == "}" {
            ensure!(
                stack.len() > 1,
                "unexpected closing brace at line {}",
                line_number + 1
            );
            let child = stack.pop().unwrap();
            stack.last_mut().unwrap().children.push(child);
        } else {
            let cmd =
                command(line).with_context(|| format!("sound-script line {}", line_number + 1))?;
            stack.last_mut().unwrap().commands.push(cmd);
        }
    }
    ensure!(stack.len() == 1, "unclosed sound-script block");
    Ok(stack.pop().unwrap())
}
fn value<'a>(block: &'a Block, key: &str) -> Option<&'a str> {
    block
        .commands
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}
fn label(header: &str) -> &str {
    header.split(':').next().unwrap_or(header)
}
fn id(header: &str) -> Result<u32> {
    let part = header
        .split_once(':')
        .context("sound definition has no id")?
        .1;
    Ok(part.split(['(', '[']).next().unwrap().parse()?)
}
fn argument(header: &str) -> Option<&str> {
    header
        .split_once('(')
        .and_then(|(_, s)| s.strip_suffix(')'))
}
fn all_commands<'a>(block: &'a Block, key: &str, out: &mut Vec<&'a str>) {
    for (k, v) in &block.commands {
        if k == key {
            out.push(v)
        }
    }
    for child in &block.children {
        all_commands(child, key, out)
    }
}
fn first_recursive<'a>(block: &'a Block, key: &str) -> Option<&'a str> {
    value(block, key).or_else(|| block.children.iter().find_map(|c| first_recursive(c, key)))
}
fn unquote(s: &str) -> String {
    s.strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(s)
        .to_owned()
}

pub fn read_sound_definitions(text: &str, name: impl Into<String>) -> Result<SoundDefinitions> {
    let name = name.into();
    let tree = parse(text)?;
    let kind = if tree
        .children
        .iter()
        .any(|b| label(&b.header) == "LcbHeader")
    {
        SoundDefinitionKind::Library
    } else if tree.children.iter().any(|b| label(&b.header) == "SIF_Type") {
        SoundDefinitionKind::Selectors
    } else {
        ensure!(
            tree.children
                .iter()
                .any(|b| label(&b.header) == "CsbHeader"),
            "unrecognized sound definitions"
        );
        SoundDefinitionKind::Events
    };
    let mut result = SoundDefinitions {
        name,
        kind,
        default_language: None,
        groups: vec![],
        banks: vec![],
        levels: vec![],
        bindings: vec![],
        events: vec![],
        resources: vec![],
        selectors: vec![],
        fidelity_notes: vec![],
        reconstruction_gaps: vec![],
    };
    for block in &tree.children {
        match label(&block.header) {
            "LcbHeader" => {
                result.default_language = value(block, "SetDefaultLanguage").map(str::to_owned)
            }
            "SndEventGroupList" => {
                for (k, v) in &block.commands {
                    ensure!(
                        k == "LoadEventGroup",
                        "unsupported library group operation {k}"
                    );
                    let (name, id) = v.rsplit_once(',').context("invalid event group")?;
                    result.groups.push(SoundGroup {
                        name: name.trim().to_owned(),
                        id: id.trim().parse()?,
                    });
                }
            }
            "SndBankE" => {
                let mut events = Vec::new();
                all_commands(block, "LoadEvent", &mut events);
                result.banks.push(SoundBankSelection {
                    id: id(&block.header)?,
                    events: events.into_iter().map(str::to_owned).collect(),
                });
            }
            "SndMapE" => {
                let mut events = Vec::new();
                all_commands(block, "LoadEvent", &mut events);
                result.levels.push(LevelSoundSelection {
                    level: block
                        .header
                        .split_once(':')
                        .context("named level header lacks ':'")?
                        .1
                        .to_owned(),
                    events: events.into_iter().map(str::to_owned).collect(),
                });
            }
            "SndEventIdList" | "SndResIdList" => {
                let is_event = label(&block.header) == "SndEventIdList";
                for (key, args) in &block.commands {
                    ensure!(
                        key == if is_event { "SetEvent" } else { "SetResource" },
                        "unsupported binding command"
                    );
                    let mut fields = args.rsplitn(3, ',');
                    let member = fields
                        .next()
                        .context("binding member missing")?
                        .trim()
                        .parse()?;
                    let bank = fields
                        .next()
                        .context("binding bank missing")?
                        .trim()
                        .parse()?;
                    let reference = fields
                        .next()
                        .context("binding reference missing")?
                        .trim()
                        .to_owned();
                    result.bindings.push(SoundDefinitionBinding {
                        reference,
                        bank,
                        member,
                        is_event,
                    });
                }
            }
            "SndEventM" => {
                let event_id = id(&block.header)?;
                let editor = tree.children.iter().find(|b| {
                    label(&b.header) == "SndEventE" && id(&b.header).ok() == Some(event_id)
                });
                let event_name = editor
                    .and_then(|b| value(b, "SetName"))
                    .unwrap_or("")
                    .to_owned();
                let mut attributes = vec![(
                    "action".into(),
                    argument(&block.header).unwrap_or("").to_owned(),
                )];
                for (k, v) in &block.commands {
                    let key = match k.as_str() {
                        "SetParam1" => "target",
                        "SetParam2" => "parameter2",
                        "SetParam3" => "parameter3",
                        "SetParam4" => "parameter4",
                        _ => k.as_str(),
                    };
                    attributes.push((key.to_owned(), v.clone()));
                }
                result.events.push(SoundEvent {
                    name: event_name,
                    resource_id: Some(event_id),
                    attributes,
                });
            }
            "SndResourceM" => {
                let resource_id = id(&block.header)?;
                let editor = tree.children.iter().find(|b| {
                    label(&b.header) == "SndResourceE" && id(&b.header).ok() == Some(resource_id)
                });
                let disk = block
                    .children
                    .iter()
                    .find(|c| label(&c.header) == "SndResourceDisk");
                let mut members = Vec::new();
                all_commands(block, "LoadResource", &mut members);
                result.resources.push(SoundResourceDefinition {
                    id: resource_id,
                    name: editor
                        .and_then(|b| value(b, "SetName"))
                        .unwrap_or("")
                        .to_owned(),
                    kind: disk
                        .and_then(|b| argument(&b.header))
                        .unwrap_or("unresolved")
                        .to_owned(),
                    file: first_recursive(block, "SetFileName").map(unquote),
                    volume: first_recursive(block, "SetVolume")
                        .map(str::parse)
                        .transpose()?,
                    looped: first_recursive(block, "SetLoop") == Some("TRUE"),
                    loop_region: if first_recursive(block, "SetLoop") == Some("TRUE") {
                        first_recursive(block, "SetLoopData")
                            .map(|value| -> Result<Option<SampleLoop>> {
                                let (start, length) =
                                    value.split_once(',').context("invalid sample loop data")?;
                                let start: u64 = start.trim().parse()?;
                                let length: u64 = length.trim().parse()?;
                                Ok(if length > 0 {
                                    Some(SampleLoop {
                                        start_frame: start,
                                        end_frame: start
                                            .checked_add(length)
                                            .context("sample loop overflow")?,
                                    })
                                } else {
                                    None
                                })
                            })
                            .transpose()?
                            .flatten()
                    } else {
                        None
                    },
                    sample_rate: first_recursive(block, "SetSampleFrequency")
                        .map(str::parse)
                        .transpose()?,
                    channels: first_recursive(block, "SetSampleChannelNumber")
                        .map(str::parse)
                        .transpose()?,
                    members: members.into_iter().map(str::to_owned).collect(),
                });
            }
            "SIF_Type" => {
                let (_, rest) = block
                    .header
                    .split_once(':')
                    .context("SIF type header lacks ':'")?;
                let name = rest.split('[').next().unwrap().to_owned();
                let type_id = argument(&block.header)
                    .context("SIF type lacks numerical id")?
                    .parse()?;
                let mut values = vec![];
                for child in &block.children {
                    ensure!(label(&child.header) == "SIF_Value", "unsupported SIF child");
                    values.push((
                        child
                            .header
                            .split_once(':')
                            .context("SIF value header lacks ':'")?
                            .1
                            .to_owned(),
                        value(child, "SetValue")
                            .context("SIF value missing")?
                            .parse()?,
                    ));
                }
                result.selectors.push(SoundSelector {
                    name,
                    id: type_id,
                    values,
                });
            }
            "CsbHeader" | "SndEventE" | "SndResourceE" => {}
            unknown => {
                result
                    .reconstruction_gaps
                    .push(AudioReconstructionGap::SoundScriptBlock {
                        name: unknown.into(),
                    });
                result.fidelity_notes.push(format!(
                    "sound-script block {unknown} has no semantic interpretation"
                ));
            }
        }
    }
    if matches!(result.kind, SoundDefinitionKind::Events) {
        result
            .reconstruction_gaps
            .push(AudioReconstructionGap::EventBehavior);
        result.fidelity_notes.push("Named events/resources imported; detailed theme transitions, flags and binary-bank event \
            compilation remain unsupported".into());
    }
    Ok(result)
}

/// Rebuild supported library/selector sidecars from current names and values.
/// CSB export is rejected until its full behavioral semantics are implemented.
pub fn write_sound_definitions(value: &SoundDefinitions) -> Result<String> {
    ensure!(
        value.reconstruction_gaps.is_empty(),
        "cannot reconstruct unresolved sound definitions"
    );
    let mut out = String::new();
    match value.kind {
        SoundDefinitionKind::Selectors => {
            for selector in &value.selectors {
                ensure!(
                    !selector.name.contains(['\n', '\r', '{', '}']),
                    "invalid selector name"
                );
                out.push_str(&format!(
                    "{{SIF_Type:{}[%ld]({})\n",
                    selector.name, selector.id
                ));
                for (name, id) in &selector.values {
                    ensure!(
                        !name.contains(['\n', '\r', '{', '}']),
                        "invalid selector value name"
                    );
                    out.push_str(&format!(
                        "    {{SIF_Value:{name}\n        SetValue[%ld]({id})\n    }}\n"
                    ));
                }
                out.push_str("}\n");
            }
        }
        SoundDefinitionKind::Library => {
            out.push_str("{LcbHeader:\n");
            if let Some(language) = &value.default_language {
                ensure!(
                    !language.contains(['\n', '\r', '(', ')']),
                    "invalid language"
                );
                out.push_str(&format!("    SetDefaultLanguage({language})\n"));
            }
            let next_group = value
                .groups
                .iter()
                .map(|g| g.id)
                .max()
                .unwrap_or(0)
                .checked_add(1)
                .context("sound group IDs leave no next free u32 ID")?;
            out.push_str(&format!("    VersionNumber(1450)\n    SetNextFreeGroupId[%lu]({next_group})\n}}\nVersionNumber(1450)\n{{SndEventGroupList:\n"));
            for group in &value.groups {
                ensure!(
                    !group.name.contains(['\n', '\r', '(', ')', ',']),
                    "invalid group name"
                );
                out.push_str(&format!(
                    "    LoadEventGroup({},{})\n",
                    group.name, group.id
                ));
            }
            out.push_str("}\n");
            for bank in &value.banks {
                out.push_str(&format!("{{SndBankE:{}\n", bank.id));
                for event in &bank.events {
                    ensure!(
                        !event.contains(['\n', '\r', '(', ')']),
                        "invalid event reference"
                    );
                    out.push_str(&format!("    LoadEvent({event})\n"));
                }
                out.push_str("}\n");
            }
            for level in &value.levels {
                ensure!(
                    !level.level.contains(['\n', '\r', '{', '}']),
                    "invalid level name"
                );
                out.push_str(&format!("{{SndMapE:{}\n", level.level));
                for event in &level.events {
                    ensure!(
                        !event.contains(['\n', '\r', '(', ')']),
                        "invalid event reference"
                    );
                    out.push_str(&format!("    LoadEvent({event})\n"));
                }
                out.push_str("}\n");
            }
            for is_event in [true, false] {
                out.push_str(if is_event {
                    "{SndEventIdList:\n"
                } else {
                    "{SndResIdList:\n"
                });
                for binding in value.bindings.iter().filter(|b| b.is_event == is_event) {
                    ensure!(
                        !binding.reference.contains(['\n', '\r', '(', ')', ',']),
                        "invalid binding reference"
                    );
                    out.push_str(&format!(
                        "    {}({},{},{})\n",
                        if is_event { "SetEvent" } else { "SetResource" },
                        binding.reference,
                        binding.bank,
                        binding.member
                    ));
                }
                out.push_str("}\n");
            }
        }
        SoundDefinitionKind::Events => bail!("CSB behavioral reconstruction is not implemented"),
    }
    Ok(out)
}

pub struct SoundScript {
    pub name: String,
}
impl FormatCodec for SoundScript {
    type Value = SoundDefinitions;
    fn support(&self) -> FormatSupport {
        FormatSupport {
            format: "CSB/LCB/SIF",
            reads: true,
            writes: true,
            notes: "Latin-1 text import; semantic LCB/SIF output, CSB event behavior export remains unsupported",
        }
    }
    fn read(&self, bytes: &[u8]) -> Result<SoundDefinitions> {
        let text = bytes.iter().map(|&b| char::from(b)).collect::<String>();
        read_sound_definitions(&text, self.name.clone())
    }
    fn write(&self, value: &SoundDefinitions) -> Result<Vec<u8>> {
        let text = write_sound_definitions(value)?;
        text.chars()
            .map(|c| {
                ensure!(
                    (c as u32) <= 255,
                    "sound script character {c:?} is outside source Latin-1 encoding"
                );
                Ok(c as u8)
            })
            .collect()
    }
}
