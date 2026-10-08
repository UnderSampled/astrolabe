//! Source-only camera ownership and byte reconstruction; never launches an engine.
use super::camera::CameraLinksWire;
use super::{container::wire, records::*, session::SourceSession};
use anyhow::{Result, ensure};

#[test]
#[ignore = "requires local Hype game data; included by tools/test.py --local"]
fn source_camera_parameter_ownership_and_target() -> Result<()> {
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "disc".into());
    let session = SourceSession::open(&game, &crate::model::Selection::hype("manoir", "en")?)?;
    let tree = session.tree()?;
    let node = tree.iter().find(|n| n.path == "world/0/2").unwrap();
    let perso = session.required(node.address, 4)?;
    let root = session.required(perso, 20)?;
    let links: CameraLinksWire = wire(session.bytes(root, 220)?)?;
    ensure!(
        super::container::encoded(&links)? == session.bytes(root, 220)?,
        "camera root declaration differs"
    );
    ensure!(
        super::container::u32_at(&session.block(root)?.data, root.offset - 4)? * 4 - 4 == 220,
        "camera allocation payload differs"
    );
    let camera = super::camera::import(&session, root, session.pointer(perso, 8)?.is_some())?;
    ensure!(
        camera.source.allocation == 23339,
        "source camera component identity differs"
    );
    ensure!(
        camera.clip_planes == [0.25, 10000.],
        "source camera clip planes differ"
    );
    for (index, pointer) in links.parameters.iter().enumerate() {
        ensure!(*pointer != 0, "missing camera parameter record");
        let a = session.required(root, index * 4)?;
        ensure!(
            super::container::u32_at(&session.block(a)?.data, a.offset - 4)? * 4 - 4 == 164,
            "camera parameter allocation payload differs"
        );
        let original = session.bytes(a, 164)?;
        let parameters = &camera.parameters[index];
        ensure!(
            parameters.source.allocation == 23340 + index as u32,
            "parameter identity differs"
        );
        ensure!(
            super::camera::write_camera_parameters(parameters)? == original,
            "camera parameters {index} reconstruction differs"
        );
        let mut edited = parameters.clone();
        edited.horizontal_fov = 1.2;
        edited.target_offset[0] = 2.;
        let bytes = super::camera::write_camera_parameters(&edited)?;
        ensure!(
            bytes[0x8c..0x90] == 1.2f32.to_le_bytes() && bytes[0xc..0x10] == 2f32.to_le_bytes(),
            "camera edit lost"
        );
        ensure!(
            bytes[..0xc] == original[..0xc]
                && bytes[0x10..0x8c] == original[0x10..0x8c]
                && bytes[0x90..] == original[0x90..],
            "camera edit changed unrelated fields"
        );
        ensure!(
            super::camera::read_camera_parameters(&bytes, parameters.source.clone())? == edited,
            "edited framing reconstruction differs"
        );
        for offset in (0..164).step_by(4) {
            ensure!(
                !session.relocated(Address {
                    offset: a.offset + offset,
                    ..a
                })?,
                "unexpected camera parameter pointer at {offset}"
            );
        }
    }
    let target = session.required(root, 16)?;
    let target_header: SuperObjectWire = wire(session.bytes(target, 64)?)?;
    // The file target is not a valid rooted manor SuperObject. The engine
    // initializes its target in 00440820; importing this word as a Perso
    // reference would silently select an unbound file address.
    ensure!(
        target_header.kind != 4 && !tree.iter().any(|n| n.address == target),
        "source target binding needs reassessment"
    );
    ensure!(
        camera.parameters[1].horizontal_fov == 1.5
            && camera.parameters[1].distance == [3.5, 4.]
            && camera.parameters[1].pitch == 0.3
            && camera.parameters[1].actor_offset == [0., 0., 1.8],
        "initial camera framing differs"
    );
    Ok(())
}
