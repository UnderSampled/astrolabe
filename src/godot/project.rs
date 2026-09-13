//! Staged project construction, engine validation, and publication.
use super::engine::strip_uids;
use super::{Manifest, quoted, run_godot};
use crate::{
    files::write_atomic,
    model::{AssetPath, Selection},
};
use anyhow::{Result, ensure};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs,
    path::PathBuf,
    time::Duration,
};
/// A fresh staging project keeps failed conversion from damaging an existing project.
pub struct Project {
    pub(super) scene_fog: Option<Option<crate::model::environment::Fog>>,
    pub(super) scene_fog_conflict: bool,
    pub(super) scene_ambient: Option<[f32; 3]>,
    pub(super) output: PathBuf,
    pub(super) stage: tempfile::TempDir,
    pub(super) selection: Selection,
    pub(super) builder: String,
    pub(super) scene: String,
    pub(super) preview_instances: Vec<String>,
    pub(super) instance_count: usize,
    pub(super) meshes: BTreeSet<AssetPath>,
    pub(super) sound_count: usize,
    pub(super) texture_count: usize,
    /// Derived PNGs, emitted only when a material enables the source color key.
    pub(super) texture_color_keys: BTreeMap<AssetPath, Option<Vec<u8>>>,
    pub(super) generated: BTreeSet<String>,
    pub(super) diagnostics: Vec<String>,
}
impl Project {
    pub fn new(output: impl Into<PathBuf>, selection: Selection) -> Result<Self> {
        let output = output.into();
        let stage = tempfile::tempdir()?;
        // Godot 4.7.2 can crash on concurrent WAV imports. The sound-program
        // regression retains two identical waveforms and exercises serial import.
        let settings = r#"[application]
config/name="Astrolabe Development"
run/main_scene="res://generated/hype/preview.tscn"
[editor]
import/use_multiple_threads=false
[display]
window/size/viewport_width=960
window/size/viewport_height=720
[rendering]
renderer/rendering_method="gl_compatibility"
renderer/rendering_method.mobile="gl_compatibility"
"#;
        write_atomic(&stage.path().join("project.godot"), settings.as_bytes())?;
        Ok(Self {
            scene_ambient: None,
            scene_fog: None,
            scene_fog_conflict: false,
            output,
            stage,
            selection,
            builder: include_str!("scripts/build.gd").into(),
            scene: String::new(),
            preview_instances: Vec::new(),
            instance_count: 0,
            meshes: BTreeSet::new(),
            sound_count: 0,
            texture_count: 0,
            texture_color_keys: BTreeMap::new(),
            generated: BTreeSet::new(),
            diagnostics: Vec::new(),
        })
    }
    /// Reserve every output before either Rust or Godot writes it. This also
    /// creates directories for instance scenes that share a mesh elsewhere.
    pub(super) fn reserve(&mut self, path: &str) -> Result<()> {
        ensure!(
            self.generated.insert(path.to_owned()),
            "duplicate Godot output {path}"
        );
        let full = self.stage.path().join(path);
        fs::create_dir_all(full.parent().expect("generated file has a parent"))?;
        Ok(())
    }
    pub(super) fn write(&mut self, path: String, bytes: &[u8]) -> Result<()> {
        self.reserve(&path)?;
        write_atomic(&self.stage.path().join(path), bytes)
    }
    pub fn diagnostic(&mut self, note: impl Into<String>) {
        self.diagnostics.push(note.into());
    }
    /// Build and validate native Godot resources, then publish only generated files.
    pub fn finish(mut self, godot: &str) -> Result<Manifest> {
        let version = run_godot(
            godot,
            self.stage.path(),
            &["--version"],
            Duration::from_secs(10),
        )?
        .trim()
        .to_string();
        ensure!(
            version.starts_with("4."),
            "Godot 4 required, found {version}"
        );
        let mut preview = format!(
            "[gd_scene load_steps={} format=3]\n\n{}\n",
            self.scene.lines().count() + 3,
            self.scene
        );
        preview.push_str("[ext_resource type=\"Script\" path=\"res://generated/hype/preview.gd\" id=\"preview\"]\n\n");
        preview.push_str(
            r#"[sub_resource type="Environment" id="environment"]
background_mode = 1
background_color = Color(0.08, 0.1, 0.14, 1)
ambient_light_source = 3
ambient_light_color = Color(1, 1, 1, 1)
ambient_light_energy = 0.7

[node name="AstrolabePreview" type="Node3D"]
script = ExtResource("preview")
PREVIEW_ASSETS
[node name="Environment" type="WorldEnvironment" parent="."]
environment = SubResource("environment")
"#,
        );
        if let Some(color) = self.scene_ambient {
            preview = preview
                .replace(
                    "ambient_light_color = Color(1, 1, 1, 1)",
                    &format!(
                        "ambient_light_color = Color({}, {}, {}, 1)",
                        color[0].clamp(0., 1.),
                        color[1].clamp(0., 1.),
                        color[2].clamp(0., 1.)
                    ),
                )
                .replace("ambient_light_energy = 0.7", "ambient_light_energy = 1.0");
        }
        if !self.scene_fog_conflict
            && let Some(Some(fog)) = &self.scene_fog
        {
            preview = preview.replace(
                "ambient_light_source = 3",
                &format!(
                    "{}ambient_light_source = 3",
                    super::environment::preview_properties(fog)
                ),
            );
        }
        for (n, id) in self.preview_instances.iter().enumerate() {
            writeln!(
                preview,
                "\n[node name=\"Model{n}\" parent=\".\" instance=ExtResource(\"{id}\")]"
            )?;
        }
        let texture_paths = self
            .generated
            .iter()
            .filter(|p| p.ends_with(".png"))
            .map(|p| quoted(&format!("res://{p}")))
            .collect::<Vec<_>>();
        let sound_paths = self
            .generated
            .iter()
            .filter_map(|p| p.strip_suffix(".wav"))
            .map(|p| quoted(&format!("res://{p}.tres")))
            .collect::<Vec<_>>();
        preview = preview.replace(
            "PREVIEW_ASSETS",
            &format!(
                "textures = PackedStringArray({})\nsounds = PackedStringArray({})\n",
                texture_paths.join(", "),
                sound_paths.join(", ")
            ),
        );
        self.write(
            "generated/hype/preview.gd".into(),
            include_bytes!("preview.gd"),
        )?;
        // Camera belongs to the development harness and is positioned after the scene loads.

        fs::create_dir_all(self.stage.path().join("generated/hype"))?;
        // The editor resolves main_scene during its first import, before native meshes exist.
        write_atomic(
            &self.stage.path().join("generated/hype/preview.tscn"),
            b"[gd_scene format=3]\n\n[node name=\"Loading\" type=\"Node3D\"]\n",
        )?;
        self.builder.push_str(r#"    var file = FileAccess.open("res://generated/hype/validation.json", FileAccess.WRITE)
    file.store_string(JSON.stringify(evidence, "  "))
    file.close()
    quit(0)
"#);
        write_atomic(&self.stage.path().join("build.gd"), self.builder.as_bytes())?;
        run_godot(
            godot,
            self.stage.path(),
            &["--headless", "--import"],
            Duration::from_secs(60),
        )?;
        run_godot(
            godot,
            self.stage.path(),
            &["--headless", "--script", "res://build.gd"],
            Duration::from_secs(60),
        )?;
        self.write("generated/hype/preview.tscn".into(), preview.as_bytes())?;
        self.reserve("generated/hype/validation.json")?;
        // ResourceSaver may assign cache-derived UIDs. Cross-resource links use explicit paths.
        for path in &self.generated {
            if path.ends_with(".tscn") || path.ends_with(".tres") {
                let full = self.stage.path().join(path);
                let text = fs::read_to_string(&full)?;
                let normalized = strip_uids(&text);
                write_atomic(&full, normalized.as_bytes())?;
            }
        }
        run_godot(
            godot,
            self.stage.path(),
            &["--headless", "--import"],
            Duration::from_secs(60),
        )?;
        let manifest = super::publication::publish(
            self.stage.path(),
            &self.output,
            self.generated,
            self.diagnostics,
            version,
        )?;
        // Import caches are derived in the destination, not copied from a temporary path.
        // This makes the fresh project immediately runnable with `godot --path`.
        run_godot(
            godot,
            &self.output,
            &["--headless", "--import"],
            Duration::from_secs(60),
        )?;
        Ok(manifest)
    }
}
