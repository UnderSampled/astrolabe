use anyhow::Result;
use astrolabe::{
    godot,
    model::{
        AssetPath, AssetRef, Selection,
        collision::CollisionProperties,
        geometry::{Material, TextureInfo, VisualMaterial},
        io::{Load, Save},
        texture::{
            OpenSpaceColorKey, Texture, TextureColors, TextureLevel, TexturePixels, TextureUsage,
        },
    },
    rete,
};

#[test]
#[ignore = "requires local Godot"]
fn godot_shared_texture_cutouts_and_blending_survive_rete_edits() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let selection = Selection::hype("material-test", "en")?;
    let id = AssetRef::new("texture:/shared/key.gf")?;
    let mut texture = Texture {
        usage: TextureUsage::Screen,
        colors: TextureColors::PaletteRgb(vec![[0, 0, 0], [0, 0, 0], [255, 0, 0]]),
        levels: vec![TextureLevel {
            width: 4,
            height: 1,
            pixels: TexturePixels::Indices(vec![0, 1, 2, 0]),
        }],
        openspace_color_key: OpenSpaceColorKey {
            rgb: [84, 45, 24],
            palette_index: 1,
        },
    };
    let mut store = rete::Store::new(temp.path().join("rete"), selection.clone())?;
    store.save(&id, &texture)?;
    // Edit meaningful key identity through Rete, without altering pixel samples.
    texture.openspace_color_key.palette_index = 0;
    store.save(&id, &texture)?;
    drop(store);
    let mut store = rete::Store::new(temp.path().join("rete"), selection.clone())?;
    let loaded = store.load(&id)?;
    assert_eq!(*loaded, texture);
    let output = temp.path().join("godot");
    let mut project = godot::Project::new(&output, selection)?;
    project.save(&id, &loaded)?;
    for (name, flags, material_flags) in [
        ("opaque", 0, 0x20000209),
        ("cutout", 2, 0x20000209),
        ("shared-cutout", 2, 0x20000209),
        ("alpha", 8, 0x20000209),
        ("additive", 0x20, 0x20000209),
        ("disabled-blending", 8, 0x20000009),
    ] {
        let material = Material {
            path: AssetPath::new(format!("level:/materials/{name}"))?,
            visual: Some(VisualMaterial {
                texture: Some(id.path.clone()),
                texture_info: Some(TextureInfo {
                    source_name: None,
                    stored_size: [4, 1],
                    logical_size: [4, 1],
                    flags,
                    addressing: 3,
                }),
                ambient: [1.; 4],
                diffuse: [0.; 4],
                specular: [0.; 4],
                color: [1.; 4],
                flags: material_flags,
                scroll_speed: [0.; 2],
                scroll_position: [0.; 2],
                scroll_mode: 0,
            }),
            sound_material: 0,
            collision: CollisionProperties::Absent,
        };
        if name == "cutout" {
            // A derived filename may coincide with another authored asset's
            // name. Reject the conflict in both traversal orders.
            let conflicting = AssetRef::<Texture>::new("texture:/shared/key.gf.color-key")?;
            for derived_first in [false, true] {
                let mut collision = godot::Project::new(
                    temp.path().join("collision"),
                    Selection::hype("material-test", "en")?,
                )?;
                collision.save(&id, &loaded)?;
                if derived_first {
                    collision.write_material(&material)?;
                    assert!(collision.save(&conflicting, &loaded).is_err());
                } else {
                    collision.save(&conflicting, &loaded)?;
                    assert!(collision.write_material(&material).is_err());
                }
            }
        }
        project.write_material(&material)?;
    }
    let engine = std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into());
    let manifest = project.finish(&engine)?;
    assert_eq!(
        manifest
            .files
            .keys()
            .filter(|p| p.ends_with("color-key.png"))
            .count(),
        1
    );
    std::fs::write(
        output.join("check.gd"),
        r#"extends SceneTree
func require(ok: bool, message: String) -> bool:
    if not ok:
        push_error(message)
        quit(1)
    return ok
func _initialize(): call_deferred("check")
func check():
    var prefix = "res://generated/hype/levels/material-test/materials/"
    var opaque = load(prefix + "opaque.tres")
    var key = load(prefix + "cutout.tres")
    var shared = load(prefix + "shared-cutout.tres")
    var alpha = load(prefix + "alpha.tres")
    var additive = load(prefix + "additive.tres")
    var disabled = load(prefix + "disabled-blending.tres")
    for material in [opaque, key, shared, alpha, additive, disabled]:
        if not require(material.texture_filter == BaseMaterial3D.TEXTURE_FILTER_LINEAR, "Hype bilinear sampling lost on a material mode"): return
    if not require(opaque.transparency == 0, "opaque material changed"): return
    if not require(disabled.transparency == 2 and is_equal_approx(disabled.alpha_scissor_threshold, 8.0/255.0), "disabling blending incorrectly disabled the source alpha test"): return
    if not require(key.transparency == 2 and key.alpha_scissor_threshold == 0.5, "cutout is not depth-writing alpha scissor"): return
    if not require(alpha.transparency == 1 and additive.transparency == 1 and additive.blend_mode == 1, "source alpha/additive mode lost"): return
    if not require(key.albedo_texture == shared.albedo_texture and key.albedo_texture != opaque.albedo_texture, "keyed and opaque sharing conflated"): return
    var pixels = key.albedo_texture.get_image()
    if not require(pixels.get_pixel(0,0).a == 0 and pixels.get_pixel(3,0).a == 0, "selected palette key stayed opaque"): return
    if not require(pixels.get_pixel(1,0) == Color.BLACK, "opaque duplicate black entry was erased"): return
    if not require(pixels.get_pixel(2,0) == Color.RED, "non-key color changed"): return
    if not require(opaque.albedo_texture.get_image().get_pixel(0,0).a == 1, "key leaked into opaque use"): return
    quit(0)
"#,
    )?;
    godot::run_godot(
        &engine,
        &output,
        &["--headless", "--script", "res://check.gd"],
        std::time::Duration::from_secs(30),
    )?;
    assert_eq!(*store.load(&id)?, texture);
    Ok(())
}
