//! Local source identity and Rete coverage for the shared Fix-owned Global actor.
use anyhow::{Context, Result, ensure};
use astrolabe::{
    model::{
        AssetRef, Selection,
        ai::{AiModel, DesignerValue, SourceRole},
        character::Perso,
        io::Load,
        scene::Scene,
    },
    openspace, rete,
};

fn game() -> std::path::PathBuf {
    std::env::var_os("ASTROLABE_GAME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("disc"))
}

#[test]
#[ignore = "requires local Hype game data; included by tools/test.py --local"]
fn fix_global_actor_brain_and_source_identity_survive_rete() -> Result<()> {
    let mut imported = openspace::content::import_scene(&game(), Selection::hype("manoir", "en")?)?;
    let scene_ref = AssetRef::<Scene>::new("level:/scene")?;
    let scene = imported.load(&scene_ref)?;
    ensure!(
        scene.global_actors.len() == 1,
        "expected one shared Fix Global actor"
    );
    let global = &scene.global_actors[0];
    ensure!(
        global.source.role == SourceRole::Fix,
        "Global identity is not Fix-owned"
    );
    ensure!(
        global.source.block == [5, 0],
        "Global identity has wrong source block"
    );
    ensure!(
        global.source.allocation == 15,
        "Global identity has wrong allocation"
    );
    ensure!(
        global.perso.path.as_str() == "fix:/persos/15",
        "Global path differs"
    );
    let perso: std::sync::Arc<Perso> = imported.load(&global.perso)?;
    ensure!(perso.name == "Global", "Fix Global Perso name differs");
    let brain = perso
        .brain
        .as_ref()
        .context("Global Brain was not imported")?;
    ensure!(
        brain.model.path.as_str() == "fix:/ai/models/1",
        "Global model differs"
    );
    let model: std::sync::Arc<AiModel> = imported.load(&brain.model)?;
    ensure!(
        model.variables.len() == 238,
        "Global designer field count differs"
    );
    ensure!(
        matches!(model.variables[17].default, DesignerValue::Boolean(true)),
        "Global v17 default differs"
    );

    let temp = tempfile::tempdir()?;
    astrolabe::app::save_rete(&mut imported, temp.path())?;
    let (mut store, _) = rete::Store::open(temp.path())?;
    let reopened_scene = store.load(&scene_ref)?;
    ensure!(
        *reopened_scene == *scene,
        "Global source identity changed in Rete"
    );
    let reopened_perso: std::sync::Arc<Perso> = store.load(&global.perso)?;
    ensure!(*reopened_perso == *perso, "Global Perso changed in Rete");
    let reopened_model: std::sync::Arc<AiModel> = store.load(&brain.model)?;
    ensure!(*reopened_model == *model, "Global AI model changed in Rete");
    Ok(())
}

#[test]
#[ignore = "requires local Hype game data; included by tools/test.py --local"]
fn rooted_source_camera_association_survives_rete() -> Result<()> {
    let mut imported = openspace::content::import_scene(&game(), Selection::hype("manoir", "en")?)?;
    let camera_ref = AssetRef::<Perso>::new("level:/persos/world/0/2")?;
    let camera = imported.load(&camera_ref)?;
    ensure!(camera.name == "StdCamer", "source camera actor differs");
    ensure!(camera.has_camera, "source camera association was dropped");
    let ordinary_ref = AssetRef::<Perso>::new("level:/persos/world/0/1")?;
    let ordinary = imported.load(&ordinary_ref)?;
    ensure!(!ordinary.has_camera, "ordinary actor gained a camera");

    let temp = tempfile::tempdir()?;
    astrolabe::app::save_rete(&mut imported, temp.path())?;
    let (mut store, _) = rete::Store::open(temp.path())?;
    ensure!(
        store.load(&camera_ref)?.has_camera,
        "Rete lost source camera association"
    );
    Ok(())
}
