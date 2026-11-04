//! A level's source loading vignette becomes a small native resource beside its
//! scene, so the persistent loading presenter can show it before the level loads.
use super::{Project, disk_path, quoted};
use crate::model::{
    AssetPath,
    content::ContentSource,
    io::{Content, Save},
    loading::LoadingVignette,
    texture::Texture,
};
use anyhow::Result;
use std::{collections::BTreeSet, fmt::Write as _};

pub(super) const RESOURCE: &str = "loading.tres";
const SCRIPT: &str = "res://addons/astrolabe/ui/loading/vignette.gd";

pub(super) fn export(
    project: &mut Project,
    source: &mut impl ContentSource,
    value: &LoadingVignette,
    written: &mut BTreeSet<AssetPath>,
) -> Result<()> {
    value.validate()?;
    let image = source.load(&value.image)?;
    image.validate_reference(&value.image.path)?;
    if written.insert(value.image.path.clone()) {
        Save::<Texture>::save(project, &value.image, &image)?;
    }
    let backdrop = format!(
        "res://{}",
        disk_path(&project.selection, &value.image.path, "png")
    );
    let bar = match &value.bar {
        Some(bar) => {
            // Bitmap bar pixels equal to the GF colour key leave the backdrop
            // visible, for both the frame and the fill (0050dc70, 00478f30).
            let path = disk_path(&project.selection, &bar.image.path, "loading-keyed.png");
            if !project.generated.contains(&path) {
                let texture = source.load(&bar.image)?;
                texture.validate_reference(&bar.image.path)?;
                project.retain_texture_mips(&bar.image, &texture)?;
                let rgba = super::material::keyed_rgba(&texture, &texture.rgba8(0)?);
                project.write(
                    path.clone(),
                    &super::material::png(texture.width(), texture.height(), &rgba)?,
                )?;
            }
            Some(format!("res://{path}"))
        }
        None => {
            project.diagnostic(format!(
                "{}: drawn default loading bar is not presented; only bitmap bars are supported",
                project.selection.level
            ));
            None
        }
    };
    let [x, y, w, h] = value.bar_art_rect();
    let mut text = format!(
        "[gd_resource type=\"Resource\" load_steps={} format=3]\n\n",
        3 + usize::from(bar.is_some())
    );
    writeln!(
        text,
        "[ext_resource type=\"Script\" path={} id=\"script\"]",
        quoted(SCRIPT)
    )?;
    writeln!(
        text,
        "[ext_resource type=\"Texture2D\" path={} id=\"backdrop\"]",
        quoted(&backdrop)
    )?;
    if let Some(bar) = &bar {
        writeln!(
            text,
            "[ext_resource type=\"Texture2D\" path={} id=\"bar\"]",
            quoted(bar)
        )?;
    }
    writeln!(
        text,
        "\n[resource]\nscript = ExtResource(\"script\")\nbackdrop = ExtResource(\"backdrop\")"
    )?;
    if bar.is_some() {
        writeln!(text, "bar = ExtResource(\"bar\")")?;
    }
    writeln!(
        text,
        "bar_rect = Rect2i({x}, {y}, {w}, {h})\nframe_y = {}\nfill_y = {}\nfill_min_x = {}\nfill_max_x = {}\nmax_value = {}\nmetadata/source_image = {}",
        value.frame_y,
        value.fill_y,
        value.min_x,
        value.max_x,
        value.max_value,
        quoted(&value.image_name)
    )?;
    let path = project.level_path(RESOURCE);
    project.write(path, text.as_bytes())
}
