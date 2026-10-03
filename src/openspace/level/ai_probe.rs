//! Original Hype records, canonical/Rete meaning, and rebuilt script operands.
use super::*;
use crate::model::ai::Expression;

#[test]
#[ignore = "requires local game data; opening initialization source evidence"]
fn source_hype_opening_particle_component_and_presentation() -> Result<()> {
    // 00542340 resolves the acting Perso's particle component at +0x40.
    // 00450fd0 assigns its selected generator at +0; 00450c30 gives size 16.
    #[derive(BinRead, BinWrite)]
    #[brw(little)]
    struct ParticleActorWire {
        prefix: [u8; 64],
        component: u32,
    }
    #[derive(BinRead, BinWrite)]
    #[brw(little)]
    struct ParticleComponentWire {
        generator: u32,
        unknown_4: [u8; 4],
        enabled: u8,
        unknown_9: [u8; 7],
    }
    // 0044fd30's registered ParticleGenerator reader binds PrtSrc, PrtSys,
    // PrtEnv at +0/+4/+8. 00451ed0 allocates 44 bytes for this definition.
    #[derive(BinRead, BinWrite)]
    #[brw(little)]
    struct GeneratorWire {
        source: u32,
        system: u32,
        environment: u32,
        unknown: [u8; 32],
    }
    // 004c1900 compares capacity/active count at +0/+2 and dispatches kind +0x1c.
    #[derive(BinRead, BinWrite)]
    #[brw(little)]
    struct ParticleSystemWire {
        capacity: u16,
        active: u16,
        unknown_4: [u8; 24],
        kind: u16,
        unknown_30: [u8; 10],
    }
    fn nodes(script: &SourceScript) -> Result<Vec<NodeWire>> {
        script
            .bytes
            .as_chunks::<8>()
            .0
            .iter()
            .map(|b| wire(b))
            .collect()
    }
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "disc".into());
    let s = SourceSession::open(&game, &Selection::hype("manoir", "en")?)?;
    let tree = s.tree()?;
    let model_for = |path: &str| -> Result<(Address, Address)> {
        let node = tree
            .iter()
            .find(|n| n.path == path)
            .context("opening actor missing")?;
        let perso = s.required(node.address, 4)?;
        let brain = s.required(perso, 16)?;
        let mind = s.required(brain, 0)?;
        Ok((perso, s.required(mind, 0)?))
    };
    let (perso, model) = model_for("world/0/0")?;
    let actor: ParticleActorWire = wire(s.bytes(perso, 68)?)?;
    assert_ne!(
        actor.component, 0,
        "the original API's absent-component skip does not apply to Hype"
    );
    let component = s.required(perso, 64)?;
    let component_bytes = s.bytes(component, 16)?;
    let state: ParticleComponentWire = wire(component_bytes)?;
    assert_eq!((state.generator, state.enabled), (0, 0));
    let mut rebuilt = std::io::Cursor::new(Vec::new());
    state.write(&mut rebuilt)?;
    assert_eq!(rebuilt.into_inner(), component_bytes);
    let normals = source_set(&s, model, 0)?.context("Hype normals missing")?;
    let init = normals.behaviors[0]
        .schedule
        .as_ref()
        .context("Hype init missing")?;
    let init_nodes = nodes(init)?;
    assert_eq!((init_nodes[14].kind, init_nodes[14].parameter), (4, 194));
    assert_eq!((init_nodes[15].kind, init_nodes[15].indent), (37, 2));
    assert_eq!((init_nodes[16].kind, init_nodes[16].parameter), (4, 193));
    assert_eq!((init_nodes[814].kind, init_nodes[814].parameter), (4, 187));
    assert_eq!((init_nodes[815].kind, init_nodes[815].parameter), (12, 255));
    assert_eq!((init_nodes[816].kind, init_nodes[816].parameter), (4, 185));
    assert_eq!((init_nodes[817].kind, init_nodes[817].parameter), (4, 267));
    let generator = s.required(add(init.address, 15 * 8), 0)?;
    let generator_bytes = s.bytes(generator, 44)?;
    let definition: GeneratorWire = wire(generator_bytes)?;
    assert!(definition.source != 0 && definition.system != 0 && definition.environment != 0);
    let mut rebuilt = std::io::Cursor::new(Vec::new());
    definition.write(&mut rebuilt)?;
    assert_eq!(rebuilt.into_inner(), generator_bytes);
    let mut records = Vec::new();
    // Native constructors 004c1170/004c4c30/004c4e20 give these exact extents.
    for (field, size, name) in [
        (0, 132, "source"),
        (4, 40, "system"),
        (8, 68, "environment"),
    ] {
        let record = s.required(generator, field)?;
        if field == 4 {
            let system: ParticleSystemWire = wire(s.bytes(record, size)?)?;
            assert_eq!((system.capacity, system.active, system.kind), (200, 0, 4));
        }
        records.push(serde_json::json!({"kind":name,"identity":s.source_identity(record)?,"bytes":s.bytes(record,size)?}));
    }
    let (_, world_model) = model_for("world/0/3")?;
    let world_normal = source_set(&s, world_model, 0)?.context("World normal missing")?;
    let world_init = world_normal.behaviors[2]
        .schedule
        .as_ref()
        .context("World initialization missing")?;
    let world_nodes = nodes(world_init)?;
    assert_eq!((world_nodes[0].kind, world_nodes[0].parameter), (4, 411));
    assert_eq!((world_nodes[22].kind, world_nodes[22].parameter), (2, 24));
    assert_eq!((world_nodes[24].kind, world_nodes[24].parameter), (4, 266));
    let world_receiver = s.required(add(world_init.address, 23 * 8), 0)?;
    let world_hidden_actor = tree
        .iter()
        .find(|n| s.pointer(n.address, 4).ok().flatten() == Some(world_receiver))
        .context("World NoDisplay receiver isn't rooted")?;
    let report = serde_json::json!({
        "scope":"unchanged manor source initialization evidence; no engine execution or presentation acceptance",
        "hype_particle_component":s.source_identity(component)?,
        "initial_selected_generator":null,"initial_component_enabled":false,
        "selection_node":14,"generator_operand_category":37,"disable_node":16,
        "selected_generator":s.source_identity(generator)?,"generator_bytes":generator_bytes,"records":records,
        "next_hype_presentation_nodes":[{"node":814,"procedure":187},{"node":816,"procedure":185},{"node":817,"procedure":267}],
        "world_normal2_node22_actual_call":{"procedure":266,"receiver":world_hidden_actor.path},
        "world_normal2_first_nodes":world_nodes.iter().take(40).enumerate().map(|(i,n)|serde_json::json!({"node":i,"category":n.kind,"parameter":n.parameter,"indent":n.indent})).collect::<Vec<_>>()
    });
    println!("{}", serde_json::to_string_pretty(&report)?);
    if let Some(path) = std::env::var_os("ASTROLABE_OPENING_SOURCE_REPORT") {
        std::fs::write(path, serde_json::to_vec_pretty(&report)?)?;
    }
    Ok(())
}

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
    assert_eq!(target_node.path, "world/0/28");
    println!("Gogoud turn: target {}, frames 5", target_node.path);
    let world = model_for("world/0/3")?;
    let reflex = source_set(&s, world, 4)?.context("World reflex set missing")?;
    let rules = &reflex.behaviors[0].rules;
    assert_eq!(rules.len(), 8);
    let mut absent_queries = 0;
    for rule in rules.iter().skip(1) {
        let rule = rule.as_ref().context("World debug rule is absent")?;
        let condition: NodeWire = wire(&rule.bytes[8..16])?;
        assert_eq!((condition.kind, condition.parameter), (1, 10));
        let node: NodeWire = wire(&rule.bytes[16..24])?;
        assert_eq!(node.kind, 14);
        assert!(
            matches!(node.parameter, 0 | u32::MAX),
            "World debug input is not actually absent"
        );
        assert!(s.pointer(add(rule.address, 16), 0)?.is_none());
        absent_queries += 1;
    }
    assert_eq!(absent_queries, 7);
    println!("World reflex0: eight populated rules, seven absent JustPressed inputs");
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
