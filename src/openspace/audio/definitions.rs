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

fn member_reference(text: &str, kind: &str) -> Result<SoundMemberRef> {
    let text = reference_argument(text)?;
    let (file, member) = text
        .trim()
        .split_once('^')
        .context("sound reference lacks owner")?;
    ensure!(
        !file.contains(['/', '\\', ':']),
        "sound definition reference must name a file"
    );
    let id = member
        .strip_prefix(kind)
        .context("sound member kind mismatch")?
        .parse()?;
    Ok(SoundMemberRef {
        definitions: crate::model::AssetRef::new(format!("sound:/Sound/{file}"))?,
        id,
    })
}
// SCR 552 allows either an unquoted reference or an [ar]("reference",n)
// argument. Keep this source syntax at the text codec boundary.
fn reference_argument(text: &str) -> Result<&str> {
    let text = text.trim();
    if let Some(rest) = text.strip_prefix('"') {
        let end = rest.find('"').context("unterminated sound reference")?;
        ensure!(
            matches!(rest[end + 1..].trim(), "" | ",n" | ", n"),
            "unsupported sound reference suffix"
        );
        Ok(&rest[..end])
    } else {
        Ok(text)
    }
}
fn selector_reference(text: &str) -> Result<SoundSelectorRef> {
    let text = reference_argument(text)?;
    let (file, name) = text
        .split_once("^SIF_Type:")
        .context("invalid sound selector reference")?;
    ensure!(
        !file.contains(['/', '\\', ':']),
        "sound selector must name a file"
    );
    Ok(SoundSelectorRef {
        definitions: crate::model::AssetRef::new(format!("sound:/Sound/{file}"))?,
        name: name.into(),
    })
}
fn selector_value(text: &str) -> Result<String> {
    Ok(reference_argument(text)?
        .rsplit_once("^SIF_Value:")
        .context("invalid sound selector value")?
        .1
        .into())
}
fn boolean(block: &Block, key: &str) -> Result<bool> {
    match value(block, key) {
        None => Ok(false),
        Some(v) if v.eq_ignore_ascii_case("FALSE") => Ok(false),
        Some(v) if v.eq_ignore_ascii_case("TRUE") => Ok(true),
        Some(other) => bail!("invalid {key} boolean {other}"),
    }
}
fn number(block: &Block, key: &str, default: f32) -> Result<f32> {
    value(block, key)
        .map(|v| unquote(v).parse())
        .transpose()
        .map(|v| v.unwrap_or(default))
        .map_err(Into::into)
}
fn operation(block: &Block) -> Result<SoundOperation> {
    let name = argument(&block.header).unwrap_or("");
    let target = || {
        value(block, "SetParam1")
            .map(|r| member_reference(r, "SndResourceM:"))
            .transpose()
    };
    Ok(match name {
        "EVT_SON_PLAY" => SoundOperation::Play {
            resource: target()?,
            pitch_variation: number(block, "SetParam2", 0.)?,
            stoppable: boolean(block, "SetStopable")?,
            dynamic: boolean(block, "SetDynamic")?,
        },
        "EVT_SON_STOP" => SoundOperation::Stop {
            event: value(block, "SetParam1")
                .map(|r| member_reference(r, "SndEventM:"))
                .transpose()?,
        },
        _ => SoundOperation::Other { name: name.into() },
    })
}
fn resource_data(block: &Block, disk: Option<&Block>) -> Result<SoundResourceData> {
    let kind = disk
        .and_then(|b| argument(&b.header))
        .unwrap_or("unresolved");
    let child = |name: &str| {
        block
            .children
            .iter()
            .find(|c| label(&c.header) == name)
            .with_context(|| format!("missing {name}"))
    };
    Ok(match kind {
        "TYPE_SAMPLE" => {
            let disk = disk.unwrap();
            SoundResourceData::Sample {
                sample: None,
                controls: SoundSampleControls {
                    pitchable: boolean(disk, "SetPitchable")?,
                    volumeable: boolean(disk, "SetVolable")?,
                    pannable: boolean(disk, "SetPanable")?,
                    spatial: boolean(disk, "SetSpacable")?,
                    reverb: boolean(disk, "SetReverbable")?,
                },
            }
        }
        "TYPE_RANDOM" => {
            let disk = disk.unwrap();
            let list = child("SndRandomListM")?;
            let mut choices = Vec::new();
            for element in &list.children {
                ensure!(
                    label(&element.header) == "SndRandomElementM",
                    "unknown random sound element"
                );
                choices.push(RandomSoundChoice {
                    resource: member_reference(
                        value(element, "LoadResource").context("random choice lacks resource")?,
                        "SndResourceM:",
                    )?,
                    probability: number(element, "SetProbability", 0.)?,
                    repeat: boolean(element, "SetElementCanBeChoosenTwice")?,
                });
            }
            SoundResourceData::Random {
                silence_probability: number(disk, "SetProbNothing", 0.)?,
                repeat_silence: boolean(disk, "SetNothingCanBeChoosenTwice")?,
                choices,
            }
        }
        "TYPE_SWITCH" => {
            let list = child("SndSwitchListM")?;
            let mut choices = Vec::new();
            for element in &list.children {
                ensure!(
                    label(&element.header) == "SndSwitchElementM",
                    "unknown switch sound element"
                );
                choices.push(SwitchSoundChoice {
                    resource: member_reference(
                        value(element, "LoadResource").context("switch choice lacks resource")?,
                        "SndResourceM:",
                    )?,
                    value: selector_value(
                        value(element, "SetInfoValue")
                            .context("switch choice lacks selector value")?,
                    )?,
                });
            }
            SoundResourceData::Switch {
                selector: selector_reference(
                    value(list, "SetInfoType").context("switch lacks selector")?,
                )?,
                default: selector_value(
                    value(list, "SetDefaultValue").context("switch lacks default")?,
                )?,
                dynamic: boolean(list, "SetDynamic")?,
                maintain: boolean(list, "SetMaintainIfSwitched")?,
                choices,
            }
        }
        "TYPE_THEME" => {
            let disk = disk.unwrap();
            let list = child("SndThemePartListM")?;
            let parts = list
                .children
                .iter()
                .map(|part| -> Result<ThemeSoundPart> {
                    ensure!(
                        label(&part.header) == "SndThemePartM",
                        "unknown theme sound part"
                    );
                    let transition =
                        match value(part, "SetTransitionType").unwrap_or("TRANSITION_CROSSFADE") {
                            "TRANSITION_ATENDOFPART" => ThemeTransition::EndOfPart,
                            "TRANSITION_CROSSFADE" => ThemeTransition::Crossfade,
                            "TRANSITION_OUTRO" => ThemeTransition::Outro,
                            other => ThemeTransition::Unknown(other.into()),
                        };
                    Ok(ThemeSoundPart {
                        resource: member_reference(
                            value(part, "LoadResource").context("theme part lacks resource")?,
                            "SndResourceM:",
                        )?,
                        transition,
                        start_loop: boolean(part, "SetStartLoop")?,
                        end_loop: boolean(part, "SetEndLoop")?,
                        loops: value(part, "SetNumberOfLoops").unwrap_or("0").parse()?,
                        transition_duration: value(part, "SetFadeDuration")
                            .unwrap_or("0")
                            .parse()?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let count: usize = value(disk, "SetNumberOfElements")
                .context("theme lacks part count")?
                .parse()?;
            ensure!(
                count == parts.len(),
                "theme part count differs from declaration"
            );
            SoundResourceData::Theme {
                start_loop: value(disk, "SetStartLoop")
                    .context("theme lacks loop entry")?
                    .parse()?,
                main_loops: value(disk, "SetNbMainLoop")
                    .context("theme lacks main loop count")?
                    .parse()?,
                parts,
            }
        }
        _ => {
            let mut members = Vec::new();
            all_commands(block, "LoadResource", &mut members);
            SoundResourceData::Other {
                kind: kind.into(),
                members: members
                    .into_iter()
                    .map(|r| member_reference(r, "SndResourceM:"))
                    .collect::<Result<_>>()?,
            }
        }
    })
}
fn emit_member(reference: &SoundMemberRef, kind: &str) -> Result<String> {
    let file = reference
        .definitions
        .path
        .file()
        .strip_prefix("Sound/")
        .context("unsupported source sound definitions directory")?;
    // The English session resolves localized owners beneath Sound/English.
    // Source LCB syntax still names only the CSB file; language chooses its folder.
    let file = file.strip_prefix("English/").unwrap_or(file);
    ensure!(
        !file.contains(['/', '\\', ':', '\n', '\r', '(', ')', ',']),
        "invalid source sound reference filename"
    );
    Ok(format!("{file}^{kind}{}", reference.id))
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
                    let target = if is_event {
                        SoundDefinitionTarget::Event(member_reference(&reference, "SndEventM:")?)
                    } else {
                        SoundDefinitionTarget::Resource(member_reference(
                            &reference,
                            "SndResourceM:",
                        )?)
                    };
                    result.bindings.push(SoundDefinitionBinding {
                        target,
                        bank,
                        member,
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
                let operation = operation(block)?;
                let mut attributes = Vec::new();
                for (k, v) in &block.commands {
                    let represented = match &operation {
                        SoundOperation::Play { .. } => matches!(
                            k.as_str(),
                            "SetParam1" | "SetParam2" | "SetStopable" | "SetDynamic"
                        ),
                        SoundOperation::Stop { .. } => k == "SetParam1",
                        SoundOperation::Other { .. } => false,
                    };
                    if !represented {
                        attributes.push((k.clone(), v.clone()));
                    }
                }
                result.events.push(SoundEvent {
                    name: event_name,
                    resource_id: Some(event_id),
                    attributes,
                    operation,
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
                result.resources.push(SoundResourceDefinition {
                    id: resource_id,
                    name: editor
                        .and_then(|b| value(b, "SetName"))
                        .unwrap_or("")
                        .to_owned(),
                    data: resource_data(block, disk)?,
                    file: first_recursive(block, "SetFileName").map(unquote),
                    volume: first_recursive(block, "SetVolume")
                        .map(str::parse)
                        .transpose()?,
                    looped: first_recursive(block, "SetLoop")
                        .is_some_and(|v| v.eq_ignore_ascii_case("TRUE")),
                    loop_region: if first_recursive(block, "SetLoop")
                        .is_some_and(|v| v.eq_ignore_ascii_case("TRUE"))
                    {
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
                for binding in value
                    .bindings
                    .iter()
                    .filter(|b| matches!(b.target, SoundDefinitionTarget::Event(_)) == is_event)
                {
                    let reference = emit_member(
                        binding.target.member(),
                        if is_event {
                            "SndEventM:"
                        } else {
                            "SndResourceM:"
                        },
                    )?;
                    out.push_str(&format!(
                        "    {}({},{},{})\n",
                        if is_event { "SetEvent" } else { "SetResource" },
                        reference,
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
