//! Native billboard meshes with a provisional source-view selector.
use super::{Project, disk_path, quoted, vec3, vector};
use crate::model::{content::ContentSource, geometry::Model, io::Content};
use anyhow::Result;
use std::fmt::Write as _;
const SCRIPT: &str = "generated/hype/runtime/sprite.gd";
impl Project {
    pub(super) fn sprite_nodes(
        &mut self,
        model: &Model,
        source: &mut impl ContentSource,
        parent: &str,
    ) -> Result<String> {
        if model.sprite_elements.is_empty() {
            return Ok(String::new());
        }
        if !self.generated.contains(SCRIPT) {
            self.write(SCRIPT.into(), include_bytes!("scripts/sprite.gd"))?;
        }
        let mut code = String::new();
        for (e, element) in model.sprite_elements.iter().enumerate() {
            for (i, placement) in element.sprites.iter().enumerate() {
                let sprite = source.load(&placement.sprite)?;
                sprite.validate_reference(&placement.sprite.path)?;
                sprite.validate()?;
                let name = format!("{parent}_sprite_{e}_{i}");
                writeln!(
                    code,
                    "    var {name} = Node3D.new()\n    {name}.name = \"Sprite_{e}_{i}\"\n    {name}.set_script(load({}))\n    {parent}.add_child({name})\n    {name}.owner = {parent}\n    {name}.position = {}\n    {name}.normal = {}",
                    quoted(&format!("res://{SCRIPT}")),
                    vec3(vector(model.positions[placement.center as usize])),
                    vec3(vector(
                        model
                            .normals
                            .get(placement.center as usize)
                            .copied()
                            .unwrap_or([0., 0., 1.])
                    ))
                )?;
                let thresholds = sprite
                    .views
                    .iter()
                    .map(|v| v.threshold.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                writeln!(
                    code,
                    "    {name}.thresholds = PackedFloat32Array([{thresholds}])"
                )?;
                for (v, view) in sprite.views.iter().enumerate() {
                    let Some(material) = &view.material else {
                        continue;
                    };
                    let mat = disk_path(&self.selection, &material.path, "tres");
                    let m = format!("{name}_view_{v}");
                    writeln!(
                        code,
                        "    var {m} = MeshInstance3D.new()\n    {m}.name = \"View_{v}\"\n    {m}.layers = 2\n    {m}.set_meta(\"view_index\", {v})\n    {m}.set_meta(\"source_sprite_mode\", {})\n    {name}.add_child({m})\n    {m}.owner = {parent}\n    var {m}_quad = QuadMesh.new()\n    {m}_quad.size = Vector2({}, {})\n    {m}_quad.center_offset = Vector3({}, {}, 0)\n    var {m}_material = load({}).duplicate()\n    {m}_material.billboard_mode = 1\n    {m}_material.billboard_keep_scale = {}\n    {m}_material.cull_mode = 2\n    {m}_quad.material = {m}_material\n    {m}.mesh = {m}_quad",
                        view.mode,
                        2. * view.half_size[0] * placement.size[0],
                        2. * view.half_size[1] * placement.size[1],
                        view.displacement[0],
                        -view.displacement[1],
                        quoted(&format!("res://{mat}")),
                        if view.mode & 1 != 0 { "true" } else { "false" }
                    )?;
                    if view.mode & !5 != 0
                        || placement.constraint != [0.; 3]
                        || placement.uv_origin_extent != [[0.; 2]; 2]
                    {
                        self.diagnostics.push(format!("{}: sprite mode {}, constraint {:?}, UV override {:?} retained; orientation/UV projection beyond camera-facing sprites is provisional",sprite.path,view.mode,placement.constraint,placement.uv_origin_extent));
                    }
                }
            }
        }
        Ok(code)
    }
}
