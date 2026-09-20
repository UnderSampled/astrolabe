//! Bake Montreal sampled hierarchies into native Godot property tracks.
//! Full affine value keys preserve shear and avoid a lossy TRS decomposition.
use super::{quoted, vector};
use crate::model::{animation::*, content::ContentSource, geometry::SourceTransform, io::Content};
use anyhow::{Context, Result, ensure};
use std::{collections::BTreeMap, fmt::Write as _};

#[derive(Clone)]
pub(super) struct Pose {
    pub transform: SourceTransform,
    pub object: Option<u16>,
    pub opacity: u8,
    pub visible: bool,
}
pub(super) fn bake(
    clip: &AnimationClip,
    source: &mut impl ContentSource,
) -> Result<Vec<BTreeMap<u16, Pose>>> {
    clip.validate()?;
    let mut locals = Vec::new();
    let mut validated = std::collections::BTreeSet::new();
    for sample in &clip.samples {
        let t = if let Some(link) = &sample.transform {
            let pool = source.load(&link.pool)?;
            pool.validate_reference(&link.pool.path)?;
            if validated.insert(pool.path.clone()) {
                pool.validate()?;
            }
            let index = pool
                .entries
                .binary_search_by_key(&link.entry, |e| e.id)
                .ok()
                .context("missing animation transform entry")?;
            pool.entries[index].value.affine()
        } else {
            SourceTransform::default()
        };
        locals.push(t);
    }
    let mut frames = Vec::new();
    for frame in &clip.frames {
        let mut parents = vec![None; frame.samples.len()];
        for &[child, parent] in &frame.hierarchy {
            // Absent logical channels cannot parent a present channel (engine sentinel).
            if clip.samples[frame.samples[parent as usize] as usize]
                .channel
                .is_some()
            {
                parents[child as usize] = Some(parent as usize);
            }
        }
        let mut poses = BTreeMap::new();
        let mut occurrences = BTreeMap::<u8, u16>::new();
        for (column, &index) in frame.samples.iter().enumerate() {
            let sample = &clip.samples[index as usize];
            let Some(id) = sample.channel else {
                continue;
            };
            let mut transform = locals[index as usize].clone();
            let mut parent = parents[column];
            while let Some(p) = parent {
                transform = compose(&locals[frame.samples[p] as usize], &transform);
                parent = parents[p];
            }
            let occurrence = occurrences.entry(id).or_default();
            let key = u16::from(id) + 256 * *occurrence;
            *occurrence += 1;
            ensure!(
                poses
                    .insert(
                        key,
                        Pose {
                            transform,
                            object: if sample.kind == ChannelKind::Object {
                                sample.object
                            } else {
                                None
                            },
                            visible: sample.visibility != 0 && sample.opacity > 0,
                            opacity: sample.opacity,
                        }
                    )
                    .is_none(),
                "{}: duplicate logical channel {id} in frame",
                clip.path
            );
        }
        frames.push(poses);
    }
    Ok(frames)
}
fn compose(a: &SourceTransform, b: &SourceTransform) -> SourceTransform {
    let mul = |left: [[f32; 3]; 3], right: [[f32; 3]; 3]| {
        std::array::from_fn(|c| {
            std::array::from_fn(|r| (0..3).map(|k| left[k][r] * right[c][k]).sum())
        })
    };
    SourceTransform {
        translation: std::array::from_fn(|r| {
            a.translation[r]
                + (0..3)
                    .map(|k| a.basis[k][r] * b.translation[k])
                    .sum::<f32>()
        }),
        basis: mul(a.basis, b.basis),
        rotation: mul(a.rotation, b.rotation),
    }
}
pub(super) fn affine(t: &SourceTransform) -> String {
    let basis = [
        vector(t.basis[0]),
        vector(t.basis[2]),
        vector(t.basis[1]).map(|v| -v),
    ];
    // The text resource Variant syntax lists matrix rows. Basis(Vector3, ...)
    // in GDScript instead takes columns; do not transpose shear/rotation keys.
    let values = (0..3)
        .flat_map(|r| (0..3).map(move |c| basis[c][r]))
        .chain(vector(t.translation))
        .map(|v| v.to_string())
        .collect::<Vec<_>>();
    format!("Transform3D({})", values.join(","))
}
/// Transient native resource writer. No source-shaped tables are shipped.
pub(super) struct NativeAnimation {
    pub text: String,
    pub tracks: usize,
}
impl NativeAnimation {
    pub fn new(name: &str, length: f64, fps: u8, frames: usize, looped: bool) -> Self {
        Self {
            tracks: 0,
            text: format!(
                "[resource]\nresource_name = {}\nlength = {length}\nloop_mode = {}\nstep = {}\nmetadata/source_fps = {fps}\nmetadata/source_frames = {frames}\n",
                quoted(name),
                u8::from(looped),
                1.0 / f64::from(fps.max(1))
            ),
        }
    }
    pub fn values(
        &mut self,
        path: &str,
        values: impl IntoIterator<Item = String>,
        fps: u8,
    ) -> Result<()> {
        let mut keys: Vec<(f64, String)> = Vec::new();
        for (index, value) in values.into_iter().enumerate() {
            if keys.last().is_none_or(|(_, v)| *v != value) {
                keys.push((index as f64 / f64::from(fps.max(1)), value));
            }
        }
        ensure!(!keys.is_empty(), "empty native track");
        let i = self.tracks;
        self.tracks += 1;
        writeln!(
            self.text,
            "tracks/{i}/type = \"value\"\ntracks/{i}/path = NodePath({})\ntracks/{i}/interp = 0\ntracks/{i}/loop_wrap = false\ntracks/{i}/keys = {{\n\"times\": PackedFloat32Array({}),\n\"transitions\": PackedFloat32Array({}),\n\"update\": 1,\n\"values\": [{}]\n}}",
            quoted(path),
            keys.iter()
                .map(|(t, _)| t.to_string())
                .collect::<Vec<_>>()
                .join(","),
            vec!["1"; keys.len()].join(","),
            keys.into_iter()
                .map(|(_, v)| v)
                .collect::<Vec<_>>()
                .join(",")
        )?;
        Ok(())
    }
    /// 00435930 integrates frame-local translation at the state's playback rate;
    /// 004436c0 supplies the resulting velocity to character mechanics. Bake that
    /// integral into native keys so Godot owns interpolation and loop extraction.
    pub fn root_translation(&mut self, clip: &AnimationClip, fps: u8) -> Result<()> {
        let mut position = [0.0f64; 3];
        let mut keys = vec!["0,1,0,0,0".to_string()];
        for (index, frame) in clip.frames.iter().enumerate() {
            for (total, displacement) in position
                .iter_mut()
                .zip(vector(frame.motion_translation.unwrap_or([0.; 3])))
            {
                *total += f64::from(displacement);
            }
            // Include the last interval's endpoint. Without it a loop loses one
            // frame of displacement and playback depends on physics tick size.
            keys.push(format!(
                "{},1,{},{},{}",
                (index + 1) as f64 / f64::from(fps.max(1)),
                position[0],
                position[1],
                position[2]
            ));
        }
        let i = self.tracks;
        self.tracks += 1;
        writeln!(
            self.text,
            "tracks/{i}/type = \"position_3d\"\ntracks/{i}/path = NodePath(\"RootMotion\")\ntracks/{i}/interp = 1\ntracks/{i}/loop_wrap = false\ntracks/{i}/keys = PackedFloat32Array({})",
            keys.join(",")
        )?;
        Ok(())
    }
    pub fn methods(&mut self, keys: Vec<(f64, String)>) -> Result<()> {
        if keys.is_empty() {
            return Ok(());
        }
        let i = self.tracks;
        self.tracks += 1;
        writeln!(
            self.text,
            "tracks/{i}/type = \"method\"\ntracks/{i}/path = NodePath(\".\")\ntracks/{i}/keys = {{\n\"times\": PackedFloat32Array({}),\n\"transitions\": PackedFloat32Array({}),\n\"values\": [{}]\n}}",
            keys.iter()
                .map(|(t, _)| t.to_string())
                .collect::<Vec<_>>()
                .join(","),
            vec!["1"; keys.len()].join(","),
            keys.into_iter()
                .map(|(_, v)| v)
                .collect::<Vec<_>>()
                .join(",")
        )?;
        Ok(())
    }
}
