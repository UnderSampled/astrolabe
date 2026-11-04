//! Model-to-Godot resource projections and instance lighting variants.
use super::{Project, disk_path, quoted, vec2, vec3, vector, whole_asset};
use crate::model::{
    AssetRef,
    audio::{AudioSamples, Sound},
    geometry::{Model, ModelInstance},
    io::{Content, Save},
    texture::Texture,
};
use anyhow::{Context, Result, ensure};
use std::fmt::Write as _;
impl Project {
    pub fn write_instance<R: crate::model::content::ContentSource>(
        &mut self,
        model: &Model,
        instance: &ModelInstance,
        resolver: &mut R,
    ) -> Result<()> {
        instance.validate_model(model)?;
        whole_asset(&model.path)?;
        whole_asset(&instance.path)?;
        let n = self.instance_count;
        self.instance_count += 1;
        // Plain instances share one mesh. Per-instance prelighting requires a
        // derived mesh variant until the full lighting adapter is implemented.
        let mesh_id = if instance.radiosity.is_empty() {
            &model.path
        } else {
            &instance.path
        };
        let mesh_path = disk_path(&self.selection, mesh_id, "mesh.tres");
        let scene_path = disk_path(&self.selection, &instance.path, "tscn");
        self.reserve(&scene_path)?;
        if self.meshes.insert(mesh_id.clone()) {
            self.reserve(&mesh_path)?;
            writeln!(self.builder, "    var mesh_{n} = ArrayMesh.new()")?;
            let mut corner_count = 0;
            // Godot rejects an empty surface. A zero-triangle source element
            // draws nothing, so it gets no surface; names keep the source index.
            let mut surface = 0usize;
            for (i, part) in model.submeshes.iter().enumerate() {
                if part.triangles.is_empty() {
                    continue;
                }
                let material = resolver.load(&part.material)?;
                material.validate_reference(&part.material.path)?;
                let material_path = disk_path(&self.selection, &material.path, "tres");
                let material = material
                    .visual
                    .as_ref()
                    .context("mesh material has no visual component")?;
                let mut vertices = Vec::new();
                let mut normals = Vec::new();
                let mut uvs = Vec::new();
                let mut colors = Vec::new();
                let mut source_rli = Vec::new();
                for (face, triangle) in part.triangles.iter().enumerate() {
                    for corner in [0, 2, 1] {
                        let index = usize::from(triangle[corner]);
                        vertices.push(vec3(vector(model.positions[index])));
                        let rli = instance.radiosity_rgba(index).unwrap_or([0.; 4]);
                        source_rli.push(format!("{},{},{},{}", rli[0], rli[1], rli[2], rli[3]));
                        let color = if let Some(rli) = instance.radiosity_rgba(index) {
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
                    r#"    var arrays_{n}_{i} = []
    arrays_{n}_{i}.resize(Mesh.ARRAY_MAX)
    arrays_{n}_{i}[Mesh.ARRAY_VERTEX] = PackedVector3Array([{}])
    arrays_{n}_{i}[Mesh.ARRAY_NORMAL] = PackedVector3Array([{}])
    arrays_{n}_{i}[Mesh.ARRAY_TEX_UV] = PackedVector2Array([{}])
    arrays_{n}_{i}[Mesh.ARRAY_COLOR] = PackedColorArray([{}])
    arrays_{n}_{i}[Mesh.ARRAY_CUSTOM0] = PackedFloat32Array([{}])
    mesh_{n}.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays_{n}_{i}, [], {{}}, Mesh.ARRAY_CUSTOM_RGBA_FLOAT << Mesh.ARRAY_FORMAT_CUSTOM0_SHIFT)
    mesh_{n}.surface_set_name({surface}, "surface_{i}")
    mesh_{n}.surface_set_material({surface}, lighting_material({}, {}))"#,
                    vertices.join(","),
                    normals.join(","),
                    uvs.join(","),
                    colors.join(","),
                    source_rli.join(","),
                    quoted(&format!("res://{material_path}")),
                    if instance.radiosity.is_empty() { 2 } else { 0 }
                )?;
                surface += 1;
            }
            if !instance.radiosity.is_empty() {
                let exact = instance
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
                self.diagnostics.push(format!("{}: static radiosity RGB projection uses 2*(ambient+diffuse*RLI/256); exact unclamped values \
            retained in mesh metadata; full lighting context remains unverified",model.path));
            }
            writeln!(
                self.builder,
                r#"    if ResourceSaver.save(mesh_{n}, {}, ResourceSaver.FLAG_CHANGE_PATH) != OK:
        push_error("cannot save generated mesh")
        quit(1)
        return
    evidence.models.append({{"path": {}, "surfaces": mesh_{n}.get_surface_count(), "corners": {corner_count}, "aabb": str(mesh_{n}.get_aabb())}})"#,
                quoted(&format!("res://{mesh_path}")),
                quoted(&model.path.to_string())
            )?;
        } else {
            writeln!(
                self.builder,
                "    var mesh_{n} = load({})",
                quoted(&format!("res://{mesh_path}"))
            )?;
        }
        // Preserve local geometry; compose the meaningful source instance transforms at its node.
        let mut transform = String::from("Transform3D.IDENTITY");
        for t in &instance.transforms {
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
        let sprites = self.sprite_nodes(model, resolver, &format!("instance_{n}"))?;
        let light_layer = if instance.radiosity.is_empty() { 2 } else { 1 };
        writeln!(
            self.builder,
            r#"    var instance_{n} = MeshInstance3D.new()
    instance_{n}.name = "Model"
    mesh_{n}.take_over_path({})
    instance_{n}.mesh = load({})
    instance_{n}.layers = {light_layer}
    instance_{n}.transform = {transform}
{sprites}
    var packed_{n} = PackedScene.new()
    if packed_{n}.pack(instance_{n}) != OK or ResourceSaver.save(packed_{n}, {}) != OK:
        push_error("cannot save model scene")
        quit(1)
        return
    instance_{n}.free()"#,
            quoted(&format!("res://{mesh_path}")),
            quoted(&format!("res://{mesh_path}")),
            quoted(&format!("res://{scene_path}"))
        )?;
        writeln!(
            self.scene,
            "[ext_resource type=\"PackedScene\" path={} id=\"model_{n}\"]",
            quoted(&format!("res://{scene_path}"))
        )?;
        self.preview_instances.push(format!("model_{n}"));
        Ok(())
    }
}
impl Save<Texture> for Project {
    fn save(&mut self, reference: &AssetRef<Texture>, texture: &Texture) -> Result<()> {
        texture.validate()?;
        whole_asset(&reference.path)?;
        let rgba = texture.rgba8(0)?;
        let png_bytes = super::material::png(texture.width(), texture.height(), &rgba)?;
        let path = disk_path(&self.selection, &reference.path, "png");
        self.write(path.clone(), &png_bytes)?;
        self.register_texture_mip_source(reference, texture)?;
        self.prepare_color_key(reference, texture)?;
        let n = self.texture_count;
        self.texture_count += 1;
        writeln!(
            self.builder,
            r#"    var image_{n} = load({}) as Texture2D
    if image_{n} == null:
        push_error("missing generated texture")
        quit(1)
        return
    evidence.textures.append({{"path": {}, "width": image_{n}.get_width(), "height": image_{n}.get_height()}})"#,
            quoted(&format!("res://{path}")),
            quoted(reference.path.as_str())
        )?;
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
        self.reserve(&resource)?;
        let n = self.sound_count;
        self.sound_count += 1;
        writeln!(
            self.builder,
            r#"    var sound_{n} = load({}) as AudioStreamWAV
    if sound_{n} == null:
        push_error("missing generated sound")
        quit(1)
        return"#,
            quoted(&format!("res://{path}"))
        )?;
        if let Some(loop_region) = &sound.loop_region {
            writeln!(
                self.builder,
                r#"    sound_{n}.loop_mode = AudioStreamWAV.LOOP_FORWARD
    sound_{n}.loop_begin = {}
    sound_{n}.loop_end = {}"#,
                loop_region.start_frame, loop_region.end_frame
            )?;
        }
        writeln!(
            self.builder,
            r#"    if ResourceSaver.save(sound_{n}, {}) != OK:
        push_error("cannot save generated sound")
        quit(1)
        return
    evidence.sounds.append({{"path": {}, "rate": sound_{n}.mix_rate, "stereo": sound_{n}.stereo, "duration": sound_{n}.get_length(), "loop_begin": sound_{n}.loop_begin, "loop_end": sound_{n}.loop_end}})"#,
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
