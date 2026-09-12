//! Godot resources derived from useful model values; no OpenSpace or Rete internals.
use crate::{
    files::write_atomic,
    model::{
        AssetPath, AssetRef, Selection,
        audio::{AudioSamples, Sound},
        geometry::{Material, Model},
        io::{Content, Load, Save},
        texture::Texture,
    },
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fmt::Write as _,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

/// Godot is Y-up and right-handed. Canonical Hype values remain untouched.
pub fn vector([x, y, z]: [f32; 3]) -> [f32; 3] {
    [x, z, -y]
}
fn vec3(v: [f32; 3]) -> String {
    format!("Vector3({},{},{})", v[0], v[1], v[2])
}
fn vec2(v: [f32; 2]) -> String {
    format!("Vector2({},{})", v[0], v[1])
}
fn quoted(value: &str) -> String {
    serde_json::to_string(value).expect("strings serialize")
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn whole_asset(id: &AssetPath) -> Result<()> {
    ensure!(
        id.member().is_none(),
        "Godot saves whole assets; member reference requires owner resolution: {id}"
    );
    Ok(())
}
fn disk_path(selection: &Selection, id: &AssetPath, extension: &str) -> String {
    let role = match id.role() {
        crate::model::PackageRole::Fix => "shared".into(),
        crate::model::PackageRole::Level => format!("levels/{}", selection.level),
        crate::model::PackageRole::Texture => "textures".into(),
        crate::model::PackageRole::Sound => "sounds".into(),
    };
    format!("generated/hype/{role}/{}.{extension}", id.file())
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Manifest {
    pub schema: u32,
    pub godot_version: String,
    pub files: BTreeMap<String, String>,
    pub diagnostics: Vec<String>,
}
/// A fresh staging project keeps failed conversion from damaging an existing project.
pub struct Project {
    output: PathBuf,
    stage: tempfile::TempDir,
    selection: Selection,
    builder: String,
    scene: String,
    mesh_count: usize,
    sound_count: usize,
    texture_count: usize,
    generated: Vec<String>,
    diagnostics: Vec<String>,
}
impl Project {
    pub fn new(output: impl Into<PathBuf>, selection: Selection) -> Result<Self> {
        let output = output.into();
        let stage = tempfile::tempdir()?;
        let settings = "[application]\nconfig/name=\"Astrolabe Development\"\nrun/main_scene=\"res://generated/hype/preview.tscn\"\n[display]\nwindow/size/viewport_width=960\nwindow/size/viewport_height=720\n[rendering]\nrenderer/rendering_method=\"gl_compatibility\"\nrenderer/rendering_method.mobile=\"gl_compatibility\"\n";
        write_atomic(&stage.path().join("project.godot"), settings.as_bytes())?;
        Ok(Self {output,stage,selection,builder:"extends SceneTree\n\nfunc _initialize():\n    call_deferred(\"build\")\n\nfunc build():\n    var evidence = {\"models\": [], \"sounds\": [], \"textures\": []}\n".into(),
            scene:String::new(),mesh_count:0,sound_count:0,texture_count:0,generated:Vec::new(),diagnostics:Vec::new()})
    }
    fn write(&mut self, path: String, bytes: &[u8]) -> Result<()> {
        ensure!(
            !self.generated.contains(&path),
            "duplicate Godot output {path}"
        );
        write_atomic(&self.stage.path().join(&path), bytes)?;
        self.generated.push(path);
        Ok(())
    }
    pub fn diagnostic(&mut self, note: impl Into<String>) {
        self.diagnostics.push(note.into());
    }
    pub fn write_material(&mut self, material: &Material) -> Result<()> {
        material.validate()?;
        whole_asset(&material.path)?;
        if let Some(texture) = &material.texture {
            whole_asset(texture)?;
        }
        let path = disk_path(&self.selection, &material.path, "tres");
        let mut resource =
            String::from("[gd_resource type=\"StandardMaterial3D\" load_steps=2 format=3]\n\n");
        if let Some(texture) = &material.texture {
            writeln!(
                resource,
                "[ext_resource type=\"Texture2D\" path={} id=\"texture\"]\n",
                quoted(&format!(
                    "res://{}",
                    disk_path(&self.selection, texture, "png")
                ))
            )?;
        }
        resource.push_str("[resource]\n");
        // Static prelit preview; complete context flags and dynamic lighting are milestone 6.
        resource
            .push_str("shading_mode = 0\ntexture_filter = 0\nvertex_color_use_as_albedo = true\n");
        if material.texture.is_some() {
            resource.push_str("albedo_texture = ExtResource(\"texture\")\n");
        }
        // The canonical fields are all retained; Godot's first material projection is explicit.
        writeln!(
            resource,
            "albedo_color = Color({}, {}, {}, {})",
            material.color[0], material.color[1], material.color[2], material.color[3]
        )?;
        self.diagnostics.push(format!("{}: static texture/radiosity preview; full context flags, dynamic lighting and blending remain unmapped",material.path));
        self.write(path, resource.as_bytes())
    }
    pub fn write_model<R: Load<Material>>(
        &mut self,
        model: &Model,
        resolver: &mut R,
    ) -> Result<()> {
        model.validate()?;
        whole_asset(&model.path)?;
        let n = self.mesh_count;
        self.mesh_count += 1;
        let mesh_path = disk_path(&self.selection, &model.path, "mesh.tres");
        let scene_path = disk_path(&self.selection, &model.path, "tscn");
        writeln!(self.builder, "    var mesh_{n} = ArrayMesh.new()")?;
        let mut corner_count = 0;
        for (i, part) in model.submeshes.iter().enumerate() {
            let material = resolver.load(&part.material)?;
            material.validate_reference(&part.material.path)?;
            let material_path = disk_path(&self.selection, &material.path, "tres");
            let mut vertices = Vec::new();
            let mut normals = Vec::new();
            let mut uvs = Vec::new();
            let mut colors = Vec::new();
            for (face, triangle) in part.triangles.iter().enumerate() {
                for corner in [0, 2, 1] {
                    let index = usize::from(triangle[corner]);
                    vertices.push(vec3(vector(model.positions[index])));
                    let color = if let Some(rli) = model.radiosity_rgba(index) {
                        std::array::from_fn::<_, 3, _>(|c| {
                            (2.0 * (material.ambient[c] + material.diffuse[c] * rli[c]))
                                .clamp(0.0, 1.0)
                        })
                    } else {
                        [1.; 3]
                    };
                    colors.push(format!(
                        "Color({}, {}, {}, 1)",
                        color[0], color[1], color[2]
                    ));
                    let normal = model
                        .normals
                        .get(index)
                        .copied()
                        .or_else(|| part.face_normals.get(face).copied())
                        .unwrap_or([0., 0., 1.]);
                    normals.push(vec3(vector(normal)));
                    let [u, v] = part.texcoords[usize::from(part.uv_indices[face][corner])];
                    uvs.push(vec2([u, 1.0 - v]));
                }
            }
            corner_count += vertices.len();
            writeln!(
                self.builder,
                "    var arrays_{n}_{i} = []\n    arrays_{n}_{i}.resize(Mesh.ARRAY_MAX)\n    arrays_{n}_{i}[Mesh.ARRAY_VERTEX] = PackedVector3Array([{}])\n    arrays_{n}_{i}[Mesh.ARRAY_NORMAL] = PackedVector3Array([{}])\n    arrays_{n}_{i}[Mesh.ARRAY_TEX_UV] = PackedVector2Array([{}])\n    arrays_{n}_{i}[Mesh.ARRAY_COLOR] = PackedColorArray([{}])\n    mesh_{n}.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays_{n}_{i})\n    mesh_{n}.surface_set_name({i}, \"surface_{i}\")\n    mesh_{n}.surface_set_material({i}, load({}))",
                vertices.join(","),
                normals.join(","),
                uvs.join(","),
                colors.join(","),
                quoted(&format!("res://{material_path}"))
            )?;
        }
        if !model.radiosity.is_empty() {
            let exact = model
                .radiosity
                .iter()
                .map(|v| {
                    format!(
                        "Color({}, {}, {}, {})",
                        v[0] as f32 / 256.0,
                        v[1] as f32 / 256.0,
                        v[2] as f32 / 256.0,
                        v[3] as f32 / 256.0
                    )
                })
                .collect::<Vec<_>>();
            writeln!(
                self.builder,
                "    mesh_{n}.set_meta(\"source_radiosity\", PackedColorArray([{}]))",
                exact.join(",")
            )?;
            self.diagnostics.push(format!("{}: static radiosity RGB projection uses 2*(ambient+diffuse*RLI/256); exact unclamped values retained in mesh metadata; full lighting context remains unverified",model.path));
        }
        fs::create_dir_all(
            self.stage
                .path()
                .join(&mesh_path)
                .parent()
                .context("mesh parent")?,
        )?;
        writeln!(
            self.builder,
            "    if ResourceSaver.save(mesh_{n}, {}, ResourceSaver.FLAG_CHANGE_PATH) != OK:\n        push_error(\"cannot save generated mesh\")\n        quit(1)\n        return\n    evidence.models.append({{\"path\": {}, \"surfaces\": mesh_{n}.get_surface_count(), \"corners\": {corner_count}, \"aabb\": str(mesh_{n}.get_aabb())}})",
            quoted(&format!("res://{mesh_path}")),
            quoted(&model.path.to_string())
        )?;
        self.generated.push(mesh_path.clone());
        // Preserve local geometry; compose the meaningful source instance transforms at its node.
        let mut transform = String::from("Transform3D.IDENTITY");
        for t in &model.instance_transforms {
            let basis = [
                vector(t.basis[0]),
                vector(t.basis[2]),
                vector(t.basis[1]).map(|v| -v),
            ];
            let converted = format!(
                "Transform3D(Basis({}, {}, {}), {})",
                vec3(basis[0]),
                vec3(basis[1]),
                vec3(basis[2]),
                vec3(vector(t.translation))
            );
            transform = format!("({transform} * {converted})");
        }
        // Godot expressions cannot appear in .tscn values; engine saves the composed transform.
        writeln!(
            self.builder,
            "    var instance_{n} = MeshInstance3D.new()\n    instance_{n}.name = \"Model\"\n    mesh_{n}.take_over_path({})\n    instance_{n}.mesh = load({})\n    instance_{n}.transform = {transform}\n    var packed_{n} = PackedScene.new()\n    if packed_{n}.pack(instance_{n}) != OK or ResourceSaver.save(packed_{n}, {}) != OK:\n        push_error(\"cannot save model scene\")\n        quit(1)\n        return\n    instance_{n}.free()",
            quoted(&format!("res://{mesh_path}")),
            quoted(&format!("res://{mesh_path}")),
            quoted(&format!("res://{scene_path}"))
        )?;
        self.generated.push(scene_path.clone());
        writeln!(
            self.scene,
            "[ext_resource type=\"PackedScene\" path={} id=\"model_{n}\"]",
            quoted(&format!("res://{scene_path}"))
        )?;
        Ok(())
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
            self.mesh_count + 3,
            self.scene
        );
        preview.push_str("[ext_resource type=\"Script\" path=\"res://generated/hype/preview.gd\" id=\"preview\"]\n\n");
        preview.push_str("[sub_resource type=\"Environment\" id=\"environment\"]\nbackground_mode = 1\nbackground_color = Color(0.08, 0.1, 0.14, 1)\nambient_light_source = 3\nambient_light_color = Color(1, 1, 1, 1)\nambient_light_energy = 0.7\n\n[node name=\"AstrolabePreview\" type=\"Node3D\"]\nscript = ExtResource(\"preview\")\nPREVIEW_ASSETS\n[node name=\"Environment\" type=\"WorldEnvironment\" parent=\".\"]\nenvironment = SubResource(\"environment\")\n");
        for n in 0..self.mesh_count {
            writeln!(
                preview,
                "\n[node name=\"Model{n}\" parent=\".\" instance=ExtResource(\"model_{n}\")]"
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
        self.builder.push_str("    var file = FileAccess.open(\"res://generated/hype/validation.json\", FileAccess.WRITE)\n    file.store_string(JSON.stringify(evidence, \"  \"))\n    file.close()\n    quit(0)\n");
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
        self.generated.push("generated/hype/validation.json".into());
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
        let old_path = self.output.join("generated/astrolabe-manifest.json");
        let old: Manifest = if old_path.exists() {
            serde_json::from_slice(&fs::read(&old_path)?)?
        } else {
            Manifest::default()
        };
        for path in old.files.keys().chain(self.generated.iter()) {
            validate_output_path(&self.output, path)?;
        }
        // All stale-file conflicts are checked before publishing anything.
        for (path, hash) in &old.files {
            let full = self.output.join(path);
            if !self.generated.contains(path) && full.exists() {
                ensure!(
                    digest(&fs::read(&full)?) == *hash,
                    "stale generated file was edited: {}",
                    full.display()
                );
            }
        }
        let mut manifest = Manifest {
            schema: 1,
            godot_version: version,
            files: BTreeMap::new(),
            diagnostics: self.diagnostics,
        };
        self.generated.sort();
        self.generated.dedup();
        // Check every collision before modifying the destination.
        for path in &self.generated {
            let destination = self.output.join(path);
            if destination.exists() {
                let expected = old.files.get(path).with_context(|| {
                    format!("refuse to overwrite unowned file {}", destination.display())
                })?;
                ensure!(
                    &digest(&fs::read(&destination)?) == expected,
                    "generated file was edited; preserve it before regeneration: {}",
                    destination.display()
                );
            }
            manifest.files.insert(
                path.clone(),
                digest(&fs::read(self.stage.path().join(path))?),
            );
        }
        for (path, hash) in &manifest.files {
            if old.files.get(path) == Some(hash) && self.output.join(path).exists() {
                continue;
            }
            write_atomic(
                &self.output.join(path),
                &fs::read(self.stage.path().join(path))?,
            )?;
        }
        if !self.output.join("project.godot").exists() {
            write_atomic(
                &self.output.join("project.godot"),
                &fs::read(self.stage.path().join("project.godot"))?,
            )?;
        }
        // Stale files are deleted only when still identical to the previous generated version.
        for (path, hash) in &old.files {
            let full = self.output.join(path);
            if !manifest.files.contains_key(path) && full.exists() {
                ensure!(
                    digest(&fs::read(&full)?) == *hash,
                    "stale generated file was edited: {}",
                    full.display()
                );
                fs::remove_file(full)?;
            }
        }
        write_atomic(&old_path, &serde_json::to_vec_pretty(&manifest)?)?;
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
impl Save<Texture> for Project {
    fn save(&mut self, reference: &AssetRef<Texture>, texture: &Texture) -> Result<()> {
        texture.validate()?;
        whole_asset(&reference.path)?;
        let rgba = texture.rgba8(0)?;
        let mut png_bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png_bytes, texture.width(), texture.height());
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header()?;
            writer.write_image_data(&rgba)?;
        }
        let path = disk_path(&self.selection, &reference.path, "png");
        self.write(path.clone(), &png_bytes)?;
        let n = self.texture_count;
        self.texture_count += 1;
        writeln!(
            self.builder,
            "    var image_{n} = load({}) as Texture2D\n    if image_{n} == null:\n        push_error(\"missing generated texture\")\n        quit(1)\n        return\n    evidence.textures.append({{\"path\": {}, \"width\": image_{n}.get_width(), \"height\": image_{n}.get_height()}})",
            quoted(&format!("res://{path}")),
            quoted(reference.path.as_str())
        )?;
        if texture.levels.len() > 1 {
            self.diagnostics.push(format!(
                "{}: original mip values retained in Rete; Godot currently imports the base image",
                reference.path
            ));
        }
        Ok(())
    }
}
impl Save<Sound> for Project {
    fn save(&mut self, reference: &AssetRef<Sound>, sound: &Sound) -> Result<()> {
        sound.validate()?;
        whole_asset(&reference.path)?;
        ensure!(
            sound.channels <= 2,
            "Godot WAV target supports mono/stereo; additional components remain in the model"
        );
        let path = disk_path(&self.selection, &reference.path, "wav");
        let spec = hound::WavSpec {
            channels: sound.channels,
            sample_rate: sound.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut bytes = std::io::Cursor::new(Vec::new());
        {
            let mut wav = hound::WavWriter::new(&mut bytes, spec)?;
            match &sound.samples {
                AudioSamples::Pcm16(samples) => {
                    for &sample in samples {
                        wav.write_sample(sample)?;
                    }
                }
                AudioSamples::Float32(_) => {
                    for sample in sound.interleaved_i16() {
                        wav.write_sample(sample)?;
                    }
                }
            }
            wav.finalize()?;
        }
        self.write(path.clone(), bytes.get_ref())?;
        let resource = disk_path(&self.selection, &reference.path, "tres");
        self.generated.push(resource.clone());
        let n = self.sound_count;
        self.sound_count += 1;
        writeln!(
            self.builder,
            "    var sound_{n} = load({}) as AudioStreamWAV\n    if sound_{n} == null:\n        push_error(\"missing generated sound\")\n        quit(1)\n        return",
            quoted(&format!("res://{path}"))
        )?;
        if let Some(loop_region) = &sound.loop_region {
            writeln!(
                self.builder,
                "    sound_{n}.loop_mode = AudioStreamWAV.LOOP_FORWARD\n    sound_{n}.loop_begin = {}\n    sound_{n}.loop_end = {}",
                loop_region.start_frame, loop_region.end_frame
            )?;
        }
        writeln!(
            self.builder,
            "    if ResourceSaver.save(sound_{n}, {}) != OK:\n        push_error(\"cannot save generated sound\")\n        quit(1)\n        return\n    evidence.sounds.append({{\"path\": {}, \"rate\": sound_{n}.mix_rate, \"stereo\": sound_{n}.stereo, \"duration\": sound_{n}.get_length(), \"loop_begin\": sound_{n}.loop_begin, \"loop_end\": sound_{n}.loop_end}})",
            quoted(&format!("res://{resource}")),
            quoted(reference.path.as_str())
        )?;
        self.diagnostics.extend(
            sound
                .fidelity_notes
                .iter()
                .map(|s| format!("{}: {s}", reference.path)),
        );
        if sound.surround.is_some() {
            self.diagnostics.push(format!(
                "{}: surround component retained in model; target mixing unverified",
                reference.path
            ));
        }
        Ok(())
    }
}
fn strip_uids(text: &str) -> String {
    let mut value = text.to_owned();
    while let Some(start) = value.find(" uid=\"uid://") {
        if let Some(end) = value[start + 6..].find('"') {
            value.replace_range(start..start + 6 + end + 1, "");
        } else {
            break;
        }
    }
    // Godot 4.7 assigns random editor node identities on PackedScene.save.
    // These provisional standalone assets use node paths, not persistent editor IDs.
    while let Some(start) = value.find(" unique_id=") {
        let end = value[start + 11..]
            .find(|c: char| !c.is_ascii_digit())
            .map_or(value.len(), |n| start + 11 + n);
        value.replace_range(start..end, "");
    }
    // ResourceSaver also chooses opaque IDs. File-local IDs only connect references;
    // name them by declaration order so independent engine processes agree.
    let declarations = value
        .lines()
        .filter_map(|line| {
            let kind = if line.starts_with("[ext_resource ") {
                "ExtResource"
            } else if line.starts_with("[sub_resource ") {
                "SubResource"
            } else {
                return None;
            };
            let start = line.find(" id=\"")? + 5;
            let end = line[start..].find('"')? + start;
            Some((kind, line[start..end].to_owned()))
        })
        .collect::<Vec<_>>();
    for (index, (kind, old)) in declarations.iter().enumerate() {
        let new = format!("stable_{}", index + 1);
        value = value.replace(&format!(" id=\"{old}\""), &format!(" id=\"{new}\""));
        value = value.replace(&format!("{kind}(\"{old}\")"), &format!("{kind}(\"{new}\")"));
    }
    value
}

/// Bounded engine process. Redirected output avoids pipe deadlocks for verbose imports.
pub fn run_godot(godot: &str, project: &Path, args: &[&str], timeout: Duration) -> Result<String> {
    let log = tempfile::tempfile()?;
    let mut child = Command::new(godot)
        .arg("--path")
        .arg(project)
        .args(args)
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log.try_clone()?))
        .spawn()
        .with_context(|| format!("start {godot}"))?;
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed() >= timeout {
            child.kill()?;
            child.wait()?;
            anyhow::bail!("Godot exceeded {}s deadline", timeout.as_secs());
        }
        thread::sleep(Duration::from_millis(50));
    };
    let mut log = log;
    std::io::Seek::rewind(&mut log)?;
    let mut output = String::new();
    log.read_to_string(&mut output)?;
    ensure!(status.success(), "Godot failed ({status}): {output}");
    ensure!(
        !output.contains("SCRIPT ERROR:") && !output.contains("ERROR:"),
        "Godot reported errors: {output}"
    );
    Ok(output)
}

fn validate_output_path(root: &Path, path: &str) -> Result<()> {
    ensure!(
        path.starts_with("generated/hype/"),
        "manifest path outside generated content: {path}"
    );
    ensure!(
        !path.contains(['\\', '\0'])
            && path
                .split('/')
                .all(|c| !c.is_empty() && c != "." && c != ".."),
        "invalid generated path: {path}"
    );
    let mut current = root.to_path_buf();
    for component in Path::new(path).components() {
        ensure!(
            matches!(component, std::path::Component::Normal(_)),
            "non-relative generated path"
        );
        current.push(component);
        if let Ok(meta) = fs::symlink_metadata(&current) {
            ensure!(
                !meta.file_type().is_symlink(),
                "generated output contains symlink: {}",
                current.display()
            );
        }
    }
    Ok(())
}
