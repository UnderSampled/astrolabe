//! Original Hype records, canonical/Rete meaning, and rebuilt script operands.
use super::*;
use crate::model::ai::Expression;

#[test]
#[ignore = "requires local game data; included by tools/test.py --local"]
fn source_ai_inventory_and_script_reconstruction() -> Result<()> {
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "disc".into());
    let selection = Selection::hype("astrolabe", "en")?;
    let imported = super::super::import_scene(&game, &selection)?;
    assert_eq!(imported.ai_models.len(), 1);
    assert_eq!(
        imported.persos.iter().filter(|p| p.brain.is_some()).count(),
        2
    );
    assert_eq!(
        imported
            .diagnostics
            .iter()
            .filter(|d| d.code == "openspace.ai.unsupported")
            .count(),
        53
    );
    assert_eq!(
        imported.scene.main_actor.as_ref().unwrap().path.as_str(),
        "level:/persos/world/0/0"
    );
    let model = &imported.ai_models[0];
    assert_eq!(model.path.as_str(), "level:/ai/models/824");
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
        for perso in imported.persos.iter().filter(|p| p.brain.is_some()) {
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
