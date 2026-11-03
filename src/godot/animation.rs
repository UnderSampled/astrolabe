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
    /// Only object samples consume the source visibility override.
    pub default_visibility: Option<bool>,
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
                            default_visibility: (sample.kind == ChannelKind::Object)
                                .then_some(sample.visibility != 0),
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
/// A source row-major rotation (acting on column vectors) as a Godot
/// quaternion [x, y, z, w]. Source (x, y, z) is Godot (x, z, -y).
fn godot_quaternion(m: &[[f64; 3]; 3]) -> [f64; 4] {
    // C maps source axes to Godot axes; the Godot rotation is C m C^T.
    let c = [[1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, -1.0, 0.0]];
    let g: [[f64; 3]; 3] = std::array::from_fn(|r| {
        std::array::from_fn(|col| {
            (0..3)
                .flat_map(|i| (0..3).map(move |j| (i, j)))
                .map(|(i, j)| c[r][i] * m[i][j] * c[col][j])
                .sum()
        })
    });
    let trace = g[0][0] + g[1][1] + g[2][2];
    let q = if trace > 0.0 {
        let s = (trace + 1.0).sqrt() * 2.0;
        [
            (g[2][1] - g[1][2]) / s,
            (g[0][2] - g[2][0]) / s,
            (g[1][0] - g[0][1]) / s,
            0.25 * s,
        ]
    } else if g[0][0] > g[1][1] && g[0][0] > g[2][2] {
        let s = (1.0 + g[0][0] - g[1][1] - g[2][2]).sqrt() * 2.0;
        [
            0.25 * s,
            (g[0][1] + g[1][0]) / s,
            (g[0][2] + g[2][0]) / s,
            (g[2][1] - g[1][2]) / s,
        ]
    } else if g[1][1] > g[2][2] {
        let s = (1.0 + g[1][1] - g[0][0] - g[2][2]).sqrt() * 2.0;
        [
            (g[0][1] + g[1][0]) / s,
            0.25 * s,
            (g[1][2] + g[2][1]) / s,
            (g[0][2] - g[2][0]) / s,
        ]
    } else {
        let s = (1.0 + g[2][2] - g[0][0] - g[1][1]).sqrt() * 2.0;
        [
            (g[0][2] + g[2][0]) / s,
            (g[1][2] + g[2][1]) / s,
            0.25 * s,
            (g[1][0] - g[0][1]) / s,
        ]
    };
    let n = q.iter().map(|v| v * v).sum::<f64>().sqrt();
    q.map(|v| v / n)
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
    let mut basis = [
        vector(t.basis[0]),
        vector(t.basis[2]),
        vector(t.basis[1]).map(|v| -v),
    ];
    // Source samples hide a channel with zero scale. Godot needs an invertible
    // basis for its global transforms, so these become imperceptibly small.
    let determinant = basis[0][0] * (basis[1][1] * basis[2][2] - basis[1][2] * basis[2][1])
        - basis[0][1] * (basis[1][0] * basis[2][2] - basis[1][2] * basis[2][0])
        + basis[0][2] * (basis[1][0] * basis[2][1] - basis[1][1] * basis[2][0]);
    if determinant == 0.0 {
        for (axis, column) in basis.iter_mut().enumerate() {
            column[axis] += 0.001;
        }
    }
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
        self.value_track(path, values, fps, 1)
    }
    /// Continuous native evaluation lets a property combine the current sample
    /// with runtime state even when consecutive source values are identical.
    pub fn continuous_values(
        &mut self,
        path: &str,
        values: impl IntoIterator<Item = String>,
        fps: u8,
    ) -> Result<()> {
        self.value_track(path, values, fps, 0)
    }
    fn value_track(
        &mut self,
        path: &str,
        values: impl IntoIterator<Item = String>,
        fps: u8,
        update: u8,
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
            "tracks/{i}/type = \"value\"\ntracks/{i}/path = NodePath({})\ntracks/{i}/interp = 0\ntracks/{i}/loop_wrap = false\ntracks/{i}/keys = {{\n\"times\": PackedFloat32Array({}),\n\"transitions\": PackedFloat32Array({}),\n\"update\": {update},\n\"values\": [{}]\n}}",
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
    /// 004436c0 also composes the frame-local root rotations since the last
    /// frame (00435db0, each frame's matrix on the left) and hands the actor
    /// their angle over the tick as an angular speed. Bake the running product
    /// as RootMotion rotation keys; clips without root rotation get no track.
    pub fn root_rotation(&mut self, clip: &AnimationClip, fps: u8) -> Result<()> {
        if clip
            .frames
            .iter()
            .all(|frame| frame.motion_rotation.is_none())
        {
            return Ok(());
        }
        let mut product = [[1.0f64, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let mut keys = vec!["0,1,0,0,0,1".to_string()];
        for (index, frame) in clip.frames.iter().enumerate() {
            if let Some(rows) = frame.motion_rotation {
                let rows = rows.map(|row| row.map(f64::from));
                product = std::array::from_fn(|r| {
                    std::array::from_fn(|c| (0..3).map(|k| rows[r][k] * product[k][c]).sum())
                });
            }
            let [x, y, z, w] = godot_quaternion(&product);
            keys.push(format!(
                "{},1,{x},{y},{z},{w}",
                (index + 1) as f64 / f64::from(fps.max(1))
            ));
        }
        let i = self.tracks;
        self.tracks += 1;
        writeln!(
            self.text,
            "tracks/{i}/type = \"rotation_3d\"\ntracks/{i}/path = NodePath(\"RootMotion\")\ntracks/{i}/interp = 1\ntracks/{i}/loop_wrap = false\ntracks/{i}/keys = PackedFloat32Array({})",
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
