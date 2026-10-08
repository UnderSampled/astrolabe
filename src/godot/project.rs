//! Staged project construction, engine validation, and publication.
use super::engine::{normalize_native, resource_uid};
use super::{Manifest, quoted, run_godot};
use crate::{
    files::write_atomic,
    model::{AssetPath, Selection},
};
use anyhow::{Context, Result, ensure};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    fs,
    path::{Component, Path, PathBuf},
    time::Duration,
};

/// One lossless PNG input for Godot's conversion-time ImageTexture writer.
/// The published texture stays a native ImageTexture resource; these inputs
/// are private to the staging project and are never published.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct TextureMipInput {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) path: String,
}
/// A fresh staging project keeps failed conversion from damaging an existing project.
pub struct Project {
    pub(super) dialogue_resources: Vec<String>,
    /// The exact emitted font bodies, checked independently of whole-level setup.
    pub(super) dialogue_font_checks: Vec<String>,
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
    /// Exact source mip images for native ImageTexture projection.
    pub(super) texture_mip_sources: BTreeMap<AssetPath, Vec<TextureMipInput>>,
    /// Source color-keyed mip images, keyed by source texture identity.
    pub(super) texture_color_key_mip_sources: BTreeMap<AssetPath, Vec<TextureMipInput>>,
    /// Native ImageTexture outputs already registered in the staged manifest.
    pub(super) texture_mip_resources: BTreeMap<(AssetPath, bool), String>,
    /// Godot conversion-time statements that save native mipmapped resources.
    pub(super) mip_texture_build_calls: Vec<String>,
    pub(super) next_mip_input: usize,
    pub(super) generated: BTreeSet<String>,
    pub(super) diagnostics: Vec<String>,
}
impl Project {
    pub fn new(output: impl Into<PathBuf>, selection: Selection) -> Result<Self> {
        let output = output.into();
        let stage = tempfile::tempdir()?;
        super::support::stage(stage.path(), &output)?;
        Ok(Self {
            dialogue_resources: Vec::new(),
            dialogue_font_checks: Vec::new(),
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
            texture_mip_sources: BTreeMap::new(),
            texture_color_key_mip_sources: BTreeMap::new(),
            texture_mip_resources: BTreeMap::new(),
            mip_texture_build_calls: Vec::new(),
            next_mip_input: 0,
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
    /// Text input for Godot's conversion-time ResourceSaver. It is never
    /// published: the owned output is the binary native resource instead.
    pub(super) fn write_build_input(&self, path: &str, bytes: &[u8]) -> Result<()> {
        ensure!(
            path.starts_with("build_inputs/")
                && Path::new(path)
                    .components()
                    .all(|part| matches!(part, Component::Normal(_))),
            "invalid build input path"
        );
        let full = self.stage.path().join(path);
        fs::create_dir_all(full.parent().expect("build input has a parent"))?;
        write_atomic(&full, bytes)
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
        let mut script_checks = String::new();
        for (i, path) in self
            .generated
            .iter()
            .filter(|p| p.ends_with(".gd"))
            .enumerate()
        {
            writeln!(
                script_checks,
                "    var checked_script_{i} = load({}) as Script\n    if checked_script_{i} == null or not checked_script_{i}.can_instantiate():\n        push_error(\"Generated script failed compilation: {path}\")\n        quit(1)\n        return",
                quoted(&format!("res://{path}")),
            )?;
        }
        // Check scripts before building any dependent native resource. A failed
        // load must not leave the builder traversing thousands of scene nodes.
        let marker = "func build():\n";
        let start = self
            .builder
            .find(marker)
            .expect("maintained builder template has build function")
            + marker.len();
        self.builder.insert_str(start, &script_checks);
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
        if !self.dialogue_font_checks.is_empty() {
            // A whole-level builder can be megabytes long. Check only the exact
            // emitted font bodies here; ordinary import/build validates the full
            // builder and native resources under their own conversion budgets.
            let preflight = tempfile::tempdir()?;
            write_atomic(
                &preflight.path().join("project.godot"),
                b"config_version=5\n",
            )?;
            let mut script = String::from("extends SceneTree\n");
            for (index, body) in self.dialogue_font_checks.iter().enumerate() {
                writeln!(script, "\nfunc check_font_{index}():\n{body}")?;
            }
            write_atomic(&preflight.path().join("font-check.gd"), script.as_bytes())?;
            let log = preflight.path().join("font-parser.log");
            run_godot(
                godot,
                preflight.path(),
                &[
                    "--headless",
                    "--log-file",
                    log.to_str().context("non-UTF-8 font parser log path")?,
                    "--check-only",
                    "--script",
                    "res://font-check.gd",
                ],
                Duration::from_secs(10),
            )
            .context("generated dialogue font syntax preflight")?;
            ensure!(
                fs::metadata(&log)?.len() <= 1024 * 1024,
                "font parser log exceeded 1MiB diagnostic limit"
            );
            let diagnostics = fs::read_to_string(&log)?;
            ensure!(
                !diagnostics.contains("SCRIPT ERROR:") && !diagnostics.contains("ERROR:"),
                "font parser log reported errors: {diagnostics}"
            );
        }
        if !self.mip_texture_build_calls.is_empty() {
            let calls = self.mip_texture_build_calls.join("\n");
            let script =
                include_str!("scripts/build_mips.gd").replace("MIP_TEXTURE_BUILDS", &calls);
            write_atomic(&self.stage.path().join("build_mips.gd"), script.as_bytes())?;
            let log = self.stage.path().join("logs/mip-parser.log");
            fs::create_dir_all(log.parent().expect("mip parser log has a parent"))?;
            run_godot(
                godot,
                self.stage.path(),
                &[
                    "--headless",
                    "--log-file",
                    log.to_str().context("non-UTF-8 mip parser log path")?,
                    "--check-only",
                    "--script",
                    "res://build_mips.gd",
                ],
                Duration::from_secs(10),
            )
            .context("mip builder syntax preflight")?;
            ensure!(
                fs::metadata(&log)?.len() <= 1024 * 1024,
                "mip parser log exceeded 1MiB diagnostic limit"
            );
            let diagnostics = fs::read_to_string(&log)?;
            ensure!(
                !diagnostics.contains("SCRIPT ERROR:") && !diagnostics.contains("ERROR:"),
                "mip parser log reported errors: {diagnostics}"
            );
            run_godot(
                godot,
                self.stage.path(),
                &["--headless", "--script", "res://build_mips.gd"],
                Duration::from_secs(60),
            )
            .context("source mip ImageTexture construction")?;
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
            // Whole levels build native meshes, animations and scenes here.
            // Dense animations are also converted to binary resources.
            Duration::from_secs(360),
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
