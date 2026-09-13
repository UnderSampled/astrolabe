//! Opt-in local fixture: source -> Godot equals source -> on-disk Rete -> Godot.
use anyhow::{Context, Result, ensure};
use astrolabe::{
    app,
    godot::run_godot,
    model::{
        AssetRef, Selection,
        geometry::{Material, Model, ModelInstance},
        io::{Load, Save},
    },
    openspace::{self, compression::decode_envelope},
    rete::Store,
};
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Seek},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

fn engine() -> String {
    std::env::var("ASTROLABE_GODOT").unwrap_or_else(|_| "godot".into())
}
fn game() -> PathBuf {
    std::env::var_os("ASTROLABE_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("disc"))
}
fn word(bytes: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        bytes
            .get(offset..offset + 4)
            .context("fragment word outside file")?
            .try_into()?,
    ))
}
fn pointer(bytes: &[u8], offset: usize) -> Result<usize> {
    let target = word(bytes, offset)?
        .checked_sub(0x1000_0000)
        .context("fragment pointer below derived base")? as usize;
    ensure!(target < bytes.len(), "fragment pointer outside file");
    Ok(target)
}
fn check_fragment(output: &Path, model: &Model, names: &BTreeSet<String>) -> Result<Vec<u8>> {
    let stem = output
        .join("fragments/level")
        .join(format!("{}/instance", model.path.file()));
    let bytes = fs::read(stem.with_extension("bin"))?;
    // Independently follow the known Montreal PO -> visual set -> LOD -> geometry chain.
    let visual = pointer(&bytes, 0)?;
    let objects = pointer(&bytes, visual + 12)?;
    let geometry = pointer(&bytes, objects)?;
    ensure!(
        u16::from_le_bytes(bytes[geometry..geometry + 2].try_into()?) == 26,
        "fragment geometry count"
    );
    let positions = pointer(&bytes, geometry + 4)?;
    let expected: Vec<u8> = model
        .positions
        .iter()
        .flatten()
        .flat_map(|v| v.to_bits().to_le_bytes())
        .collect();
    ensure!(
        bytes.get(positions..positions + expected.len()) == Some(expected.as_slice()),
        "fragment lost exact edited positions"
    );
    let rtb = fs::read(stem.with_extension("rtb"))?;
    ensure!(rtb[..3] == [1, 6, 2], "unexpected RTB owner group");
    let count = word(&rtb, 3)? as usize;
    ensure!(count == 75, "first-model relocation coverage changed");
    let (rows, used) = decode_envelope(&rtb[7..], 4096)?;
    ensure!(
        used + 7 == rtb.len() && rows.len() == count * 8,
        "RTB row extent"
    );
    for row in rows.as_chunks::<8>().0 {
        let site = word(row, 0)?
            .checked_sub(0x1000_0000)
            .context("RTB site below base")? as usize;
        let _ = pointer(&bytes, site)?;
        ensure!(row[4..] == [6, 2, 0, 0], "RTB target identity");
    }
    let ptx = fs::read(stem.with_extension("ptx"))?;
    ensure!(
        ptx.len() == 8208 && word(&ptx, 0)? == 1024 && word(&ptx, 4)? == 1024,
        "complete no-Fix PTX table"
    );
    let texture_pointers = &ptx[8..24];
    ensure!(
        ptx[24..4104].iter().all(|&b| b == 0) && word(&ptx, 4104)? == 4,
        "four populated PTX texture slots"
    );
    for index in 0..1024 {
        ensure!(
            word(&ptx, 4108 + index * 4)? == if index < 4 { 1 } else { 0 },
            "derived PTX memory channel"
        );
    }
    ensure!(word(&ptx, 8204)? == 1, "PTX current memory channel");
    let mut rebuilt_names = BTreeSet::new();
    for row in texture_pointers.as_chunks::<4>().0 {
        let texture = word(row, 0)?
            .checked_sub(0x1000_0000)
            .context("PTX pointer below base")? as usize;
        let name = bytes
            .get(texture + 100..texture + 180)
            .context("texture name extent")?;
        let end = name.iter().position(|&b| b == 0).unwrap_or(name.len());
        rebuilt_names.insert(String::from_utf8(name[..end].to_vec())?);
    }
    ensure!(
        &rebuilt_names == names,
        "canonical texture resolution overwrote source texture-info names"
    );
    let rtt = fs::read(stem.with_extension("rtt"))?;
    ensure!(rtt[..3] == [1, 5, 1] && word(&rtt, 3)? == 4, "RTT group");
    let (rows, used) = decode_envelope(&rtt[7..], 128)?;
    ensure!(used + 7 == rtt.len() && rows.len() == 32, "RTT row extent");
    for (row, entry) in rows
        .as_chunks::<8>()
        .0
        .iter()
        .zip(texture_pointers.as_chunks::<4>().0.iter())
    {
        ensure!(
            &row[..4] == entry && row[4..] == [6, 2, 0, 0],
            "RTT must identify PTX pointer values"
        );
    }
    Ok(bytes)
}
fn inspect_edited_mesh(project: &Path, model: &Model) -> Result<()> {
    let p = model.positions[usize::from(model.submeshes[0].triangles[0][0])];
    let script = format!(
        r#"extends SceneTree
func require(ok: bool, reason: String):
    if not ok:
        push_error(reason)
        quit(1)
        return false
    return true
func _initialize():
    var mesh = load("res://generated/hype/levels/astrolabe/models/world/1/0/0/instance.mesh.tres") as ArrayMesh
    if not require(mesh != null and mesh.get_surface_count() == 4, "real mesh closure"): return
    var arrays = mesh.surface_get_arrays(0)
    var vertex = arrays[Mesh.ARRAY_VERTEX][0]
    if not require(vertex.is_equal_approx(Vector3({x}, {z}, {y})), "persisted edit did not reach actual Godot vertex"): return
    var exact = mesh.get_meta("source_radiosity")
    if not require(exact.size() == 26 and exact[11].is_equal_approx(Color(1.546875, 0.1875, -0.9296875, 0)), "exact source radiosity metadata lost or clamped"): return
    var colors = arrays[Mesh.ARRAY_COLOR]
    if not require(colors.size() == arrays[Mesh.ARRAY_VERTEX].size() and is_equal_approx(colors[0].a, 1), "source RLI fourth component incorrectly became opacity"): return
    var material = mesh.surface_get_material(0) as StandardMaterial3D
    if not require(material != null and material.albedo_texture != null, "reloaded material/texture dependency missing"): return
    quit(0)
"#,
        x = p[0],
        z = p[2],
        y = -p[1]
    );
    fs::write(project.join("inspect_real_route.gd"), script)?;
    run_godot(
        &engine(),
        project,
        &["--headless", "--script", "res://inspect_real_route.gd"],
        Duration::from_secs(30),
    )?;
    Ok(())
}
fn worker() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let persisted = temp.path().join("rete");
    let direct = temp.path().join("direct");
    let through = temp.path().join("through-rete");
    let selection = Selection::hype("astrolabe", "EN")?;
    let mut imported = openspace::content::import_model(&game(), selection.clone())?;
    ensure!(
        imported.inventory.len() == 10 && imported.roots.len() == 1,
        "first-model content closure"
    );
    let roots = imported.roots.clone();
    let source_diagnostics = imported.diagnostics.clone();
    ensure!(
        source_diagnostics
            .iter()
            .any(|d| d.code == "openspace.model.partial_import"),
        "missing explicit source coverage"
    );
    let transient_root = imported
        .source_summary
        .as_ref()
        .context("source debug summary")?
        .actual_world
        .clone();
    let instance = imported.load(&AssetRef::<ModelInstance>::from_path(
        roots[0].reference.clone(),
    ))?;
    let model_ref = instance.model.clone();
    let original = (*imported.load(&model_ref)?).clone();
    ensure!(
        original.name == "world/1/0/0" && original.submeshes[0].triangles[0] == [3, 0, 14],
        "unexpected rooted source model"
    );
    app::save_rete(&mut imported, &persisted)?;
    let first = astrolabe::godot::export(&mut imported, &roots, selection, &direct, &engine())?;
    // Both import session and writer are now gone. Every subsequent asset load goes
    // through newly opened MessagePack files; no source context is passed to export.
    drop(imported);
    let (mut store, package) = Store::open(&persisted)?;
    ensure!(
        package.diagnostics == source_diagnostics,
        "Rete discarded observed source coverage"
    );
    let package_json = serde_json::to_string(&package)?;
    ensure!(
        !package_json.contains("source_summary") && !package_json.contains(&transient_root),
        "transient source addresses leaked into Rete package metadata"
    );
    let reloaded = store.load(&model_ref)?;
    ensure!(
        *reloaded == original,
        "disk Rete changed useful source values"
    );
    let material_ref = &reloaded.submeshes[0].material;
    let material: Arc<Material> = store.load(material_ref)?;
    ensure!(
        Arc::ptr_eq(&material, &store.load(material_ref)?),
        "repeated dependency load lost sharing"
    );
    let mut names = BTreeSet::new();
    for part in &reloaded.submeshes {
        let m: Arc<Material> = store.load(&part.material)?;
        names.insert(
            m.visual
                .as_ref()
                .unwrap()
                .texture_info
                .as_ref()
                .context("real material sampling")?
                .source_name
                .clone()
                .context("original texture-info name")?,
        );
    }
    let second = astrolabe::godot::export(
        &mut store,
        &package.roots,
        package.selection.clone(),
        &through,
        &engine(),
    )?;
    ensure!(
        first.files == second.files,
        "direct and closed/reopened Rete Godot outputs differ"
    );
    ensure!(
        first.diagnostics == second.diagnostics,
        "routes disagree about target coverage"
    );
    let baseline_dir = temp.path().join("reconstruction-before");
    let report = astrolabe::openspace::export(&mut store, &package.roots, &baseline_dir)?;
    ensure!(
        report.files.len() == 9,
        "model fragment, SNA framing and four dependent textures"
    );
    let framed = fs::read(baseline_dir.join("fragments/level/models/world/1/0/0/instance.sna"))?;
    ensure!(
        framed.starts_with(&[6, 2]),
        "wrong generated fragment block"
    );
    let (decoded, used) =
        astrolabe::openspace::compression::decode_envelope(&framed[22..], 64 * 1024 * 1024)?;
    ensure!(
        used + 22 == framed.len()
            && decoded
                == fs::read(baseline_dir.join("fragments/level/models/world/1/0/0/instance.bin"))?,
        "framed model differs from generated content"
    );
    let baseline = check_fragment(&baseline_dir, &original, &names)?;
    let mut edited = (*reloaded).clone();
    let index = usize::from(edited.submeshes[0].triangles[0][0]);
    edited.positions[index][0] += 1.25;
    store.save(&model_ref, &edited)?;
    drop(reloaded);
    drop(material);
    drop(store);
    let (mut store, package) = Store::open(&persisted)?;
    ensure!(
        *store.load(&model_ref)? == edited,
        "edit was not persisted to disk"
    );
    let changed = astrolabe::godot::export(
        &mut store,
        &package.roots,
        package.selection.clone(),
        &through,
        &engine(),
    )?;
    let mesh = "generated/hype/levels/astrolabe/models/world/1/0/0/instance.mesh.tres";
    ensure!(
        second.files.get(mesh) != changed.files.get(mesh),
        "persisted geometry edit did not change mesh bytes"
    );
    for (path, hash) in &second.files {
        if path.ends_with(".png") || path.contains("/materials/") {
            ensure!(
                changed.files.get(path) == Some(hash),
                "position edit changed unrelated media/material"
            );
        }
    }
    inspect_edited_mesh(&through, &edited)?;
    let changed_dir = temp.path().join("reconstruction-after");
    astrolabe::openspace::export(&mut store, &package.roots, &changed_dir)?;
    let rebuilt = check_fragment(&changed_dir, &edited, &names)?;
    ensure!(
        baseline != rebuilt,
        "source-free export ignored the persisted edit"
    );
    ensure!(
        original.positions[index][0] + 1.25 == edited.positions[index][0],
        "canonical original changed during target conversion"
    );
    Ok(())
}
/// One local invocation, including all engine children, has an eight-minute cap.
#[test]
#[ignore = "requires local Hype disc and Godot 4; included by tools/test.py --local"]
fn real_model_direct_and_reopened_rete_routes_agree() -> Result<()> {
    if std::env::var_os("ASTROLABE_REAL_ROUTES_WORKER").is_some() {
        return worker();
    }
    let mut log = tempfile::tempfile()?;
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args([
            "--exact",
            "real_model_direct_and_reopened_rete_routes_agree",
            "--ignored",
            "--nocapture",
        ])
        .env("ASTROLABE_REAL_ROUTES_WORKER", "1")
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log.try_clone()?));
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.spawn()?;
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if start.elapsed() > Duration::from_secs(480) {
            #[cfg(unix)]
            {
                let _ = Command::new("kill")
                    .args(["-KILL", "--", &format!("-{}", child.id())])
                    .status();
            }
            let _ = child.kill();
            let _ = child.wait();
            anyhow::bail!("real route test exceeded eight-minute deadline");
        }
        thread::sleep(Duration::from_millis(50));
    };
    log.rewind()?;
    let mut output = String::new();
    log.read_to_string(&mut output)?;
    ensure!(status.success(), "real route test failed:\n{output}");
    println!(
        "Real model routes passed in {:.2}s",
        start.elapsed().as_secs_f64()
    );
    Ok(())
}

#[test]
fn source_diagnostics_survive_package_reopen_without_game_data() -> Result<()> {
    use astrolabe::model::{
        AssetPath,
        io::{Diagnostic, Severity},
    };
    let temp = tempfile::tempdir()?;
    let mut imported = openspace::content::Imported::new(Selection::hype("synthetic", "EN")?);
    let expected = Diagnostic {
        code: "openspace.model.partial_import".into(),
        asset: Some(AssetPath::new("level:/models/example")?),
        severity: Severity::Warning,
        message: "The selected object has unsupported collision content.".into(),
    };
    imported.diagnostics.push(expected.clone());
    app::save_rete(&mut imported, temp.path())?;
    drop(imported);
    let (store, package) = Store::open(temp.path())?;
    ensure!(
        package.diagnostics == vec![expected.clone()],
        "source coverage was not persisted"
    );
    let json = serde_json::to_value(&package)?;
    ensure!(
        json.get("source_summary").is_none(),
        "transient debug summary became package metadata"
    );
    // Existing development packages without the new optional field remain readable.
    let mut older = json.clone();
    older.as_object_mut().unwrap().remove("diagnostics");
    let older: astrolabe::rete::Package = serde_json::from_value(older)?;
    ensure!(
        older.diagnostics.is_empty(),
        "missing old diagnostics should default to empty"
    );
    let mut invalid = package.clone();
    invalid.diagnostics[0].code.clear();
    ensure!(
        store.save_package(&invalid).is_err(),
        "invalid persistent diagnostic code accepted"
    );
    // The CLI must report the persisted source notes even though its adapter only
    // receives useful assets and has no source-import context.
    let output = temp.path().join("source-export");
    let result = Command::new(env!("CARGO_BIN_EXE_astrolabe"))
        .args(["export-openspace"])
        .arg(temp.path())
        .arg(&output)
        .output()?;
    ensure!(
        result.status.success(),
        "source report CLI failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&result.stdout)?;
    ensure!(
        report["source_diagnostics"] == serde_json::to_value(vec![expected])?,
        "CLI dropped restored source coverage"
    );
    let saved: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join("reconstruction-report.json"))?)?;
    ensure!(
        saved == report,
        "CLI and persistent reconstruction reports disagree"
    );
    Ok(())
}
