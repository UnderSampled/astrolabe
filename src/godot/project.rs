//! Staged project construction, engine validation, and publication.
use super::engine::{normalize_native, resource_uid};
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
    pub(super) has_level_scene: bool,
    pub(super) scene_fog: Option<Option<crate::model::environment::Fog>>,
    pub(super) scene_fog_conflict: bool,
    pub(super) scene_ambient: Option<[f32; 3]>,
    pub(super) output: PathBuf,
    pub(super) stage: tempfile::TempDir,
    pub(super) selection: Selection,
    pub(super) builder: String,
    pub(super) builder_functions: String,
    pub(super) character_rigs: BTreeSet<String>,
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
        super::support::stage(stage.path(), &output)?;
        Ok(Self {
            has_level_scene: false,
            scene_ambient: None,
            scene_fog: None,
            scene_fog_conflict: false,
            output,
            stage,
            selection,
            builder: include_str!("scripts/build.gd").into(),
            builder_functions: String::new(),
            character_rigs: BTreeSet::new(),
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
    pub(super) fn level_path(&self, file: &str) -> String {
        format!("generated/hype/levels/{}/{file}", self.selection.level)
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
        writeln!(
            preview,
            "[ext_resource type=\"Script\" path={} id=\"preview\"]\n",
            quoted(&format!("res://{}", self.level_path("preview.gd")))
        )?;
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
        if self.has_level_scene {
            preview = preview.replace("[node name=\"Environment\" type=\"WorldEnvironment\" parent=\".\"]\nenvironment = SubResource(\"environment\")\n", "");
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
        self.write(self.level_path("preview.gd"), include_bytes!("preview.gd"))?;
        // Camera belongs to the development harness and is positioned after the scene loads.

        fs::create_dir_all(self.stage.path().join(self.level_path("")))?;
        // The editor resolves main_scene during its first import, before native meshes exist.
        write_atomic(
            &self.stage.path().join(self.level_path("preview.tscn")),
            b"[gd_scene format=3]\n\n[node name=\"Loading\" type=\"Node3D\"]\n",
        )?;
        for path in self.generated.iter().filter(|p| p.ends_with(".gd")) {
            writeln!(
                self.builder,
                "    if not load({}).can_instantiate():\n        push_error(\"Generated script failed compilation\")\n        quit(1)\n        return",
                quoted(&format!("res://{path}"))
            )?;
        }
        for path in self
            .generated
            .iter()
            .filter(|p| p.ends_with(".tscn"))
            .cloned()
            .chain(std::iter::once(self.level_path("preview.tscn")))
        {
            writeln!(
                self.builder,
                "    expose_scene_tree({})",
                quoted(&format!("res://{path}"))
            )?;
        }
        self.builder.push_str(&format!(
            r#"    var file = FileAccess.open({}, FileAccess.WRITE)
    file.store_string(JSON.stringify(evidence, "  "))
    file.close()
    quit(0)
"#,
            quoted(&format!("res://{}", self.level_path("validation.json")))
        ));
        self.builder.push_str(&self.builder_functions);
        write_atomic(&self.stage.path().join("build.gd"), self.builder.as_bytes())?;
        // Script identity belongs to the exporter, not a random import cache.
        let scripts = self
            .generated
            .iter()
            .filter(|p| p.ends_with(".gd"))
            .cloned()
            .collect::<Vec<_>>();
        for path in scripts {
            self.write(
                format!("{path}.uid"),
                format!("{}\n", resource_uid(&path)).as_bytes(),
            )?;
        }
        run_godot(
            godot,
            self.stage.path(),
            &["--headless", "--import"],
            Duration::from_secs(60),
        )?;
        // The first import needs the placeholder; the builder can inspect the
        // real preview once it has saved all referenced native scenes.
        self.write(self.level_path("preview.tscn"), preview.as_bytes())?;
        run_godot(
            godot,
            self.stage.path(),
            &["--headless", "--script", "res://build.gd"],
            Duration::from_secs(60),
        )?;
        self.reserve(&self.level_path("validation.json"))?;
        // Preserve path-derived resource/node identities across edits and cache rebuilds.
        for path in &self.generated {
            if path.ends_with(".tscn") || path.ends_with(".tres") {
                let full = self.stage.path().join(path);
                let text = fs::read_to_string(&full)?;
                let normalized = normalize_native(path, &text)?;
                write_atomic(&full, normalized.as_bytes())?;
            }
        }
        let media = self
            .generated
            .iter()
            .filter(|p| p.ends_with(".png") || p.ends_with(".wav"))
            .cloned()
            .collect::<Vec<_>>();
        for path in media {
            let sidecar = format!("{path}.import");
            let text = fs::read_to_string(self.stage.path().join(&sidecar))?;
            let text = text
                .lines()
                .map(|line| {
                    if line.starts_with("uid=") {
                        format!("uid=\"{}\"", resource_uid(&path))
                    } else {
                        line.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
                + "\n";
            self.write(sidecar, text.as_bytes())?;
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
            &self.selection.level,
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
