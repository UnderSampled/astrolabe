//! Original Hype records, canonical/Rete meaning, and rebuilt script operands.
use super::*;
use crate::model::ai::Expression;

#[test]
#[ignore = "requires local game data; selected opening API evidence"]
fn source_opening_turn_and_absent_world_inputs_are_bound_deliberately() -> Result<()> {
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "disc".into());
    let s = SourceSession::open(&game, &Selection::hype("manoir", "en")?)?;
    let tree = s.tree()?;
    let model_for = |path: &str| -> Result<Address> {
        let node = tree
            .iter()
            .find(|n| n.path == path)
            .context("opening actor missing")?;
        let perso = s.required(node.address, 4)?;
        let brain = s.required(perso, 16)?;
        let mind = s.required(brain, 0)?;
        s.required(mind, 0)
    };
    let gogoud = model_for("world/0/5")?;
    let set = source_set(&s, gogoud, 0)?.context("Gogoud normal behavior set missing")?;
    let script = set.behaviors[0]
        .schedule
        .as_ref()
        .context("Gogoud schedule missing")?;
    let nodes = script
        .bytes
        .as_chunks::<8>()
        .0
        .iter()
        .map(|b| wire::<NodeWire>(b))
        .collect::<Result<Vec<_>>>()?;
    assert_eq!((nodes[18].kind, nodes[18].parameter), (5, 23));
    assert_eq!(
        (nodes[19].kind, nodes[20].kind, nodes[20].parameter),
        (24, 12, 5)
    );
    let target = s.required(add(script.address, 19 * 8), 0)?;
    let target_node = tree
        .iter()
        .find(|n| s.pointer(n.address, 4).ok().flatten() == Some(target))
        .context("turn target isn't a rooted Perso")?;
    println!("Gogoud turn: target {}, frames 5", target_node.path);
    let world = model_for("world/0/3")?;
    let reflex = source_set(&s, world, 4)?.context("World reflex set missing")?;
    for rule in reflex.behaviors[0].rules.iter().skip(1).flatten() {
        let node: NodeWire = wire(&rule.bytes[16..24])?;
        assert_eq!(node.kind, 14);
        assert!(
            matches!(node.parameter, 0 | u32::MAX),
            "World debug input is not actually absent"
        );
        assert!(s.pointer(add(rule.address, 16), 0)?.is_none());
    }
    assert_eq!(reflex.behaviors[0].rules.len(), 9);
    Ok(())
}

#[test]
#[ignore = "requires local game data; included by tools/test.py --local"]
fn source_manor_lever_save_identity() -> Result<()> {
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "disc".into());
    let selection = Selection::hype("manoir", "en")?;
    let s = SourceSession::open(&game, &selection)?;
    let node = s
        .tree()?
        .into_iter()
        .find(|node| node.path == "world/0/27")
        .context("manor dummy is absent")?;
    let perso = s.required(node.address, 4)?;
    let brain = s.required(perso, 16)?;
    let mind = s.required(brain, 0)?;
    let model = s.required(mind, 0)?;
    let definition = s.required(model, 8)?;
    let infos = s.required(definition, 4)?;
    let info: InfoWire = wire(s.bytes(add(infos, 8 * 16), 16)?)?;
    assert_eq!(info.kind, 0);
    assert_eq!(info.save, 0x200);
    let memory = s.required(mind, 12)?;
    let current = s.required(memory, 8)?;
    let value = add(current, info.offset as usize);
    assert_eq!(value.owner, Owner::Fix);
    assert_eq!(value.key, BlockKey(17, 0));
    assert_eq!(value.offset, 297732);
    assert_eq!(s.bytes(value, 1)?, &[0]);
    Ok(())
}

#[test]
#[ignore = "requires local game data; included by tools/test.py --local"]
fn source_inventory_magic_flags_identity() -> Result<()> {
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "disc".into());
    let selection = Selection::hype("manoir", "en")?;
    let s = SourceSession::open(&game, &selection)?;
    for (offset, expected_id, expected_kind) in
        [(1560, 54, "MagicIce"), (2800, 74, "MagicLightning")]
    {
        let active = Address {
            owner: Owner::Fix,
            key: BlockKey(10, 0),
            offset,
        };
        let handle = Address {
            offset: offset - 8,
            ..active
        };
        let member = s.required(handle, 0)?;
        let record = s.bytes(member, 30)?;
        let id = i16::from_le_bytes(record[28..30].try_into()?);
        let name = s.required(member, 12)?;
        let bytes = s.bytes(name, 32)?;
        let end = bytes
            .iter()
            .position(|byte| *byte == 0)
            .context("inventory kind name")?;
        assert_eq!(id, expected_id);
        assert_eq!(std::str::from_utf8(&bytes[..end])?, expected_kind);
        assert_eq!(s.bytes(active, 1)?, &[0]);
    }
    Ok(())
}

#[test]
#[ignore = "requires local game data; included by tools/test.py --local"]
fn source_ai_inventory_and_script_reconstruction() -> Result<()> {
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "disc".into());
    let selection = Selection::hype("astrolabe", "en")?;
    let imported = super::super::import_scene(&game, &selection)?;
    // The rooted graph includes brains of action-generated Perso templates.
    // The native ZDE condition decoder keeps model 733 in the rooted AI set.
    assert_eq!(imported.ai_models.len(), 58);
    assert!(imported.persos.iter().filter(|p| p.brain.is_some()).count() > 2);
    assert!(
        imported
            .diagnostics
            .iter()
            .any(|d| d.code == "openspace.ai.unsupported_script")
    );
    assert_eq!(
        imported.scene.main_actor.as_ref().unwrap().path.as_str(),
        "level:/persos/world/0/0"
    );
    let model = imported
        .ai_models
        .iter()
        .find(|model| model.path.as_str() == "level:/ai/models/824")
        .context("sample torch AI model was not imported")?;
    assert_eq!(model.variables[0].default, DesignerValue::Boolean(false));
    let reopened = crate::rete::decode::<AiModel>(&crate::rete::encode(model)?)?;
    assert_eq!(&reopened, model);
    let s = SourceSession::open(&game, &selection)?;
    let sounds = SoundSites::open(&game, &selection, &s)?;
    let mut models = BTreeSet::new();
    let mut scripts = BTreeSet::new();
    let mut brain_count = 0;
    let mut nodes = 0;
    for node in s.tree()? {
        if u32_at(s.bytes(node.address, 4)?, 0)? != 4 {
            continue;
        }
        let p = s.required(node.address, 4)?;
        if imported
            .persos
            .iter()
            .any(|p| p.path.file() == format!("persos/{}", node.path) && p.brain.is_some())
        {
            println!(
                "AI comparison {}: SO {}, perso {}, global matrix {}",
                node.path,
                s.label(node.address)?,
                s.label(p)?,
                s.label(s.required(node.address, 36)?)?
            );
        }
        let Some(brain) = s.pointer(p, 16)? else {
            continue;
        };
        brain_count += 1;
        let _: BrainWire = wire(allocated(&s, brain, 8)?)?;
        let mind = s.required(brain, 0)?;
        let _: MindWire = wire(allocated(&s, mind, 20)?)?;
        let a = s.required(mind, 0)?;
        if !models.insert(a) {
            continue;
        }
        let _: ModelWire = wire(allocated(&s, a, 16)?)?;
        let sets = [source_set(&s, a, 0)?, source_set(&s, a, 4)?];
        for set in sets.iter().flatten() {
            for behavior in &set.behaviors {
                for script in behavior
                    .rules
                    .iter()
                    .flatten()
                    .chain(behavior.schedule.iter())
                {
                    if scripts.insert(script.address) {
                        nodes += script.bytes.len() / 8;
                    }
                }
            }
        }
        if path(&s, a)? != model.path {
            continue;
        }
        let set = sets[0].as_ref().unwrap();
        assert_eq!(set.behaviors.len(), 1);
        let source = set.behaviors[0].schedule.as_ref().unwrap();
        let script = reopened.normal.as_ref().unwrap().behaviors[0]
            .schedule
            .as_ref()
            .unwrap();
        let mut codec = AiScriptCodec::default();
        for (i, bytes) in source.bytes.as_chunks::<8>().0.iter().enumerate() {
            let node: NodeWire = wire(bytes)?;
            if node.kind == 34 {
                let event = sounds.events[&add(source.address, i * 8)].clone();
                codec.sounds.insert(i, event.clone());
                codec.sound_handles.push((event, node.parameter));
            }
        }
        assert_eq!(codec.read(&source.bytes)?, *script);
        assert_eq!(
            codec.write(script)?,
            source.bytes,
            "typed/reopened AI reconstruction"
        );
        assert_eq!(
            script.statements[0],
            Statement::Wait {
                milliseconds: Expression::Integer(250)
            }
        );
        let mut edited = script.clone();
        edited.statements[0] = Statement::Wait {
            milliseconds: Expression::Integer(100),
        };
        let bytes = codec.write(&edited)?;
        assert_eq!(u32_at(&bytes, 8)?, 100);
        assert_eq!(&bytes[12..], &source.bytes[12..]);
        assert_eq!(&bytes[..8], &source.bytes[..8]);
        assert_eq!(codec.read(&bytes)?, edited);
        let mut changed_binding = codec.sound_handles.clone();
        for (_, handle) in &mut changed_binding {
            *handle = handle.wrapping_add(0x1000);
        }
        let moved = AiScriptCodec {
            sounds: codec.sounds.clone(),
            sound_handles: changed_binding,
            ..Default::default()
        }
        .write(script)?;
        assert_ne!(moved, source.bytes);
        assert_eq!(
            codec.read(&moved)?,
            *script,
            "SND links define meaning, not assigned runtime handles"
        );
        let torch_persos = imported
            .persos
            .iter()
            .filter(|p| {
                p.brain
                    .as_ref()
                    .is_some_and(|brain| brain.model.path == reopened.path)
            })
            .collect::<Vec<_>>();
        assert_eq!(torch_persos.len(), 2);
        for perso in torch_persos {
            let brain = perso.brain.as_ref().unwrap();
            brain.validate_model(&reopened)?;
            assert_eq!(brain.variables, vec![DesignerValue::Boolean(false)]);
            assert!(brain.initial_variables.is_none());
            assert_eq!(brain.normal.as_ref().unwrap().current, Some(0));
        }
    }
    assert_eq!(
        (brain_count, models.len(), scripts.len(), nodes),
        (55, 33, 530, 43681)
    );
    println!(
        "AI_SOURCE_OK: 55 brains, 33 models, 530 scripts; 2 independent compiled torch instances; 32-node script byte-exact after Rete"
    );
    Ok(())
}
