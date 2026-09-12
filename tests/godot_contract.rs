//! Opt-in real-engine contract probe with synthetic shared content and no OpenSpace dependency.
use anyhow::{Context, Result, anyhow, ensure};
use astrolabe::{
    app::export_godot,
    godot::{Manifest, run_godot},
    model::{
        AssetPath, AssetRef, Selection,
        geometry::{Material, Model, SourceTransform, Submesh, TextureInfo},
        io::{Content, Load},
        texture::{
            OpenSpaceColorKey, Texture, TextureColors, TextureLevel, TexturePixels, TextureUsage,
        },
    },
    rete::Root,
};
use std::{
    any::Any,
    collections::BTreeMap,
    fs,
    io::{Read, Seek},
    path::Path,
    process::{Command, Stdio},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

#[derive(Default)]
struct Memory {
    assets: BTreeMap<AssetPath, Arc<dyn Any + Send + Sync>>,
    reads: BTreeMap<AssetPath, usize>,
}
impl Memory {
    fn insert<T: Content>(&mut self, id: AssetPath, value: T) {
        self.assets.insert(id, Arc::new(value));
    }
}
impl<T: Content> Load<T> for Memory {
    fn load(&mut self, id: &AssetRef<T>) -> Result<Arc<T>> {
        *self.reads.entry(id.path.clone()).or_default() += 1;
        self.assets
            .get(&id.path)
            .context("missing synthetic dependency")?
            .clone()
            .downcast()
            .map_err(|_| anyhow!("synthetic kind mismatch"))
    }
}
fn fixture() -> Result<(Model, Material, Texture)> {
    let model_id = AssetPath::new("level:/models/quad")?;
    let material_id = AssetPath::new("level:/materials/shared")?;
    let texture_id = AssetPath::new("texture:/shared/corners.tga")?;
    let texture = Texture {
        usage: TextureUsage::Material,
        colors: TextureColors::Components { bits: [8, 8, 8, 8] },
        levels: vec![TextureLevel {
            width: 2,
            height: 2,
            pixels: TexturePixels::Components(vec![
                [255, 0, 0, 255],
                [0, 255, 0, 255],
                [0, 0, 255, 255],
                [255, 255, 255, 255],
            ]),
        }],
        openspace_color_key: OpenSpaceColorKey {
            rgb: [0; 3],
            palette_index: -1,
        },
    };
    let material = Material {
        path: material_id.clone(),
        texture: Some(texture_id),
        texture_info: Some(TextureInfo {
            source_name: None,
            stored_size: [2, 2],
            logical_size: [2, 2],
            flags: 0,
            addressing: 0,
        }),
        ambient: [0., 0., 0., 1.],
        diffuse: [0.8, 0.8, 0.8, 1.],
        specular: [0.; 4],
        color: [0.25, 0.5, 0.75, 1.],
        flags: 0,
        scroll_speed: [0.; 2],
        scroll_position: [0.; 2],
        scroll_mode: 0,
        sound_material: 0,
    };
    let part = |tri, uv| -> Result<Submesh> {
        Ok(Submesh {
            material: AssetRef::new(material_id.to_string())?,
            triangles: vec![tri],
            texcoords: vec![[0., 0.], [1., 0.], [1., 1.], [0.25, 0.75]],
            uv_indices: vec![uv],
            face_normals: vec![[0., 0., 1.]],
            vertex_indices: vec![0, 1, 2, 3],
        })
    };
    let model = Model {
        path: model_id,
        name: "quad".into(),
        lod_distance: 2.,
        positions: vec![[0., 0., 0.], [2., 0., 0.], [2., 3., 0.], [0., 3., 0.]],
        normals: vec![[0., 0., 1.]; 4],
        radiosity: vec![],
        submeshes: vec![part([0, 1, 2], [3, 0, 1])?, part([0, 2, 3], [2, 1, 0])?],
        vertex_materials: vec![],
        instance_transforms: vec![
            SourceTransform {
                translation: [5., 7., 11.],
                rotation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                basis: [[0., 2., 0.], [-3., 0., 0.], [0., 0., 4.]],
            },
            SourceTransform {
                translation: [1., 2., 3.],
                rotation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
                basis: [[1., 0., 0.], [0.5, 1., 0.], [0., 0., 1.]],
            },
        ],
    };
    Ok((model, material, texture))
}
fn source(model: &Model, material: &Material, texture: &Texture) -> Memory {
    let mut memory = Memory::default();
    memory.insert(model.path.clone(), model.clone());
    memory.insert(material.path.clone(), material.clone());
    memory.insert(material.texture.clone().unwrap(), texture.clone());
    memory
}
fn engine() -> String {
    std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into())
}
fn export(path: &Path, model: &Model, material: &Material, texture: &Texture) -> Result<Manifest> {
    let mut memory = source(model, material, texture);
    let roots = [Root {
        kind: "model".into(),
        reference: model.path.clone(),
    }];
    let manifest = export_godot(
        &mut memory,
        &roots,
        Selection::hype("synthetic", "EN")?,
        path,
        &engine(),
    )?;
    ensure!(
        memory.reads.get(material.texture.as_ref().unwrap()) == Some(&1),
        "shared texture loaded more than once"
    );
    ensure!(
        manifest
            .files
            .keys()
            .filter(|p| p.ends_with("materials/shared.tres"))
            .count()
            == 1,
        "shared material was duplicated"
    );
    Ok(manifest)
}
fn inspect(path: &Path, expected_width: f32, color: f32) -> Result<()> {
    let script = format!(
        r#"extends SceneTree
func require(ok: bool, reason: String):
    if not ok:
        push_error(reason)
        quit(1)
        return false
    return true
func _initialize():
    call_deferred("check_contract")
func check_contract():
    var scene = load("res://generated/hype/levels/synthetic/models/quad.tscn") as PackedScene
    if not require(scene != null, "missing generated scene"): return
    var node = scene.instantiate() as MeshInstance3D
    var mesh = load("res://generated/hype/levels/synthetic/models/quad.mesh.tres") as ArrayMesh
    if not require(mesh != null and node.mesh != null, "missing native mesh"): return
    if not require(node.mesh == mesh, "scene embeds or duplicates generated mesh instead of sharing it"): return
    if not require(mesh.get_surface_count() == 2, "lost material surface"): return
    var first = mesh.surface_get_arrays(0)
    var p = first[Mesh.ARRAY_VERTEX]
    var n = first[Mesh.ARRAY_NORMAL]
    var uv = first[Mesh.ARRAY_TEX_UV]
    if not require(p.size() == 3, "corner count"): return
    if not require(p[0].is_equal_approx(Vector3(0, 0, 0)), "first corner"): return
    if not require(p[1].is_equal_approx(Vector3({expected_width}, 0, -3)), "clockwise second corner"): return
    if not require(p[2].is_equal_approx(Vector3({expected_width}, 0, 0)), "clockwise third corner"): return
    if not require(uv[0].is_equal_approx(Vector2(0.25, 0.25)) and uv[1].is_equal_approx(Vector2(1,1)) and uv[2].is_equal_approx(Vector2(0,1)), "separate UV corner mapping"): return
    if not require((p[1]-p[0]).cross(p[2]-p[0]).dot(n[0]) < 0, "frontface winding disagrees with normals"): return
    if not require((node.transform * Vector3(0,0,0)).is_equal_approx(Vector3(-1,23,-9)), "composed source translation"): return
    if not require((node.transform * Vector3(1,0,0)).is_equal_approx(Vector3(-1,23,-11)), "full source basis used exactly once"): return
    var material = mesh.surface_get_material(0) as StandardMaterial3D
    if not require(material == mesh.surface_get_material(1), "lost shared material identity"): return
    if not require(is_equal_approx(material.albedo_color.r, {color}), "material color edit ignored"): return
    var image = material.albedo_texture.get_image()
    if not require(image.get_pixel(0,0).is_equal_approx(Color(0,0,1,1)) and image.get_pixel(0,1).is_equal_approx(Color(1,0,0,1)), "texture row orientation"): return
    var wrapper = load("res://authored/wrapper.tscn") as PackedScene
    if not require(wrapper != null, "authored wrapper broken"): return
    node.free()
    quit(0)
"#
    );
    fs::write(path.join("inspect_contract.gd"), script)?;
    run_godot(
        &engine(),
        path,
        &["--headless", "--import"],
        Duration::from_secs(40),
    )?;
    run_godot(
        &engine(),
        path,
        &["--headless", "--script", "res://inspect_contract.gd"],
        Duration::from_secs(30),
    )?;
    Ok(())
}
fn worker() -> Result<()> {
    let (model, material, texture) = fixture()?;
    model.validate()?;
    material.validate()?;
    texture.validate()?;
    let source_before = (model.clone(), material.clone(), texture.clone());
    let temp = tempfile::tempdir()?;
    let a = temp.path().join("a");
    let b = temp.path().join("b");
    fs::create_dir_all(a.join("authored"))?;
    let host=b"[application]\nconfig/name=\"Synthetic Authored Host\"\n[rendering]\nrenderer/rendering_method=\"gl_compatibility\"\n";
    fs::write(a.join("project.godot"), host)?;
    let first = export(&a, &model, &material, &texture)?;
    let second = export(&b, &model, &material, &texture)?;
    if first.files != second.files {
        let mut differences = String::new();
        for (path, hash) in &first.files {
            if second.files.get(path) != Some(hash) {
                differences.push_str(&format!(
                    "\n{path}:\nFIRST:\n{}\nSECOND:\n{}\n",
                    String::from_utf8_lossy(&fs::read(a.join(path))?),
                    String::from_utf8_lossy(&fs::read(b.join(path))?)
                ));
            }
        }
        anyhow::bail!("two clean outputs differ:{differences}");
    }
    let wrapper=b"[gd_scene load_steps=2 format=3]\n[ext_resource type=\"PackedScene\" path=\"res://generated/hype/levels/synthetic/models/quad.tscn\" id=\"1\"]\n[node name=\"AuthoredWrapper\" type=\"Node3D\"]\n[node name=\"GameModel\" parent=\".\" instance=ExtResource(\"1\")]\n";
    fs::write(a.join("authored/wrapper.tscn"), wrapper)?;
    inspect(&a, 2., 0.25)?;
    let mut edited = model.clone();
    edited.positions[1][0] = 4.;
    edited.positions[2][0] = 4.;
    let mut edited_material = material.clone();
    edited_material.color[0] = 0.75;
    let changed = export(&a, &edited, &edited_material, &texture)?;
    ensure!(
        first.files != changed.files,
        "deliberate edit did not change target output"
    );
    ensure!(
        fs::read(a.join("authored/wrapper.tscn"))? == wrapper,
        "authored wrapper changed"
    );
    ensure!(
        fs::read(a.join("project.godot"))? == host,
        "host project changed"
    );
    inspect(&a, 4., 0.75)?;
    // A stale generated file that the developer edited must abort before any changed
    // resource is published. The attempted export below deliberately reverts geometry.
    let live_mesh = a.join("generated/hype/levels/synthetic/models/quad.mesh.tres");
    let current_mesh = fs::read(&live_mesh)?;
    let stale = "generated/hype/retired.tres";
    fs::write(
        a.join(stale),
        b"developer edited this formerly generated resource",
    )?;
    let mut disputed = changed.clone();
    disputed
        .files
        .insert(stale.into(), "old generated content hash".into());
    let manifest_path = a.join("generated/astrolabe-manifest.json");
    fs::write(&manifest_path, serde_json::to_vec(&disputed)?)?;
    let error = export(&a, &model, &material, &texture).expect_err("edited stale file accepted");
    ensure!(
        format!("{error:#}").contains("stale generated file was edited"),
        "unexpected stale conflict: {error:#}"
    );
    ensure!(
        fs::read(&live_mesh)? == current_mesh,
        "stale conflict partially published changed mesh"
    );
    ensure!(
        fs::read(a.join("authored/wrapper.tscn"))? == wrapper,
        "stale cleanup touched authored wrapper"
    );
    // A manifest is data, never authority to delete authored files outside its namespace.
    disputed.files.remove(stale);
    disputed
        .files
        .insert("authored/wrapper.tscn".into(), "irrelevant".into());
    fs::write(&manifest_path, serde_json::to_vec(&disputed)?)?;
    let error =
        export(&a, &model, &material, &texture).expect_err("outside-namespace manifest accepted");
    ensure!(
        format!("{error:#}").contains("manifest path outside generated content"),
        "unexpected manifest conflict: {error:#}"
    );
    ensure!(
        fs::read(&live_mesh)? == current_mesh,
        "invalid manifest partially published changed mesh"
    );
    ensure!(
        fs::read(a.join("authored/wrapper.tscn"))? == wrapper,
        "manifest path deleted authored wrapper"
    );
    ensure!(
        (model, material, texture) == source_before,
        "export mutated canonical input values"
    );
    Ok(())
}
/// The entire invocation, including cold imports/builds and cleanup, has an eight-minute deadline.
/// The worker process group owns any Godot subprocesses, so timeout also cleans those up.
#[test]
#[ignore = "requires local Godot 4; bounded complete integration invocation"]
fn synthetic_godot_contract() -> Result<()> {
    if std::env::var_os("ASTROLABE_GODOT_CONTRACT_WORKER").is_some() {
        return worker();
    }
    let mut log = tempfile::tempfile()?;
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args([
            "--exact",
            "synthetic_godot_contract",
            "--ignored",
            "--nocapture",
        ])
        .env("ASTROLABE_GODOT_CONTRACT_WORKER", "1")
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log.try_clone()?));
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.spawn()?;
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if start.elapsed() > Duration::from_secs(480) {
            #[cfg(unix)]
            {
                let _ = Command::new("kill")
                    .args(["-KILL", "--", &format!("-{}", child.id())])
                    .status();
            }
            let _ = child.kill();
            let _ = child.wait();
            anyhow::bail!("Godot contract exceeded the eight-minute suite deadline");
        }
        thread::sleep(Duration::from_millis(50));
    };
    log.rewind()?;
    let mut output = String::new();
    log.read_to_string(&mut output)?;
    ensure!(status.success(), "Godot contract failed:\n{output}");
    println!(
        "Godot contract passed in {:.2}s",
        start.elapsed().as_secs_f64()
    );
    Ok(())
}

/// A resolver returning the wrong named object must fail before conversion creates
/// resources under that object's unexpected path. This does not require Godot.
#[test]
fn mismatched_content_identity_is_rejected_before_engine_start() -> Result<()> {
    let (model, material, texture) = fixture()?;
    let roots = [Root {
        kind: "model".into(),
        reference: model.path.clone(),
    }];
    let selection = Selection::hype("synthetic", "EN")?;
    let temp = tempfile::tempdir()?;
    let mut wrong_model = model.clone();
    wrong_model.path = AssetPath::new("level:/models/unexpected")?;
    let mut memory = source(&model, &material, &texture);
    memory.insert(model.path.clone(), wrong_model);
    let result = export_godot(
        &mut memory,
        &roots,
        selection.clone(),
        temp.path(),
        "engine-must-not-be-started",
    );
    ensure!(
        format!("{:#}", result.expect_err("wrong model identity accepted"))
            .contains("content identity")
    );
    let mut wrong_material = material.clone();
    wrong_material.path = AssetPath::new("level:/materials/unexpected")?;
    let mut memory = source(&model, &material, &texture);
    memory.insert(material.path.clone(), wrong_material);
    let result = export_godot(
        &mut memory,
        &roots,
        selection,
        temp.path(),
        "engine-must-not-be-started",
    );
    ensure!(
        format!(
            "{:#}",
            result.expect_err("wrong material identity accepted")
        )
        .contains("content identity")
    );
    ensure!(
        !temp.path().join("generated").exists(),
        "identity failure published generated files"
    );
    Ok(())
}
