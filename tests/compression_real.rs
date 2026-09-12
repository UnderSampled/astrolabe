//! Independent verification of every populated envelope in selected source files.
use anyhow::{Context, Result, ensure};
use astrolabe::openspace::compression::{
    DEFAULT_MAX_DECODED, compress, decode_envelope, encode_envelope,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, process::Command};

fn word(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}

#[test]
#[ignore = "requires local Hype game files and system liblzo2; run in bounded corpus suite"]
fn all_fix_astrolabe_sna_and_relocation_envelopes_match_liblzo2() -> Result<()> {
    let game = std::env::var_os("ASTROLABE_GAME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("disc"));
    let fixtures = [
        (
            "Fix.sna",
            9,
            6,
            11028210,
            "36b5557a7d9d8d1f7636fcf88f945dc056ed1484f2b169322b13561c2099360e",
        ),
        (
            "astrolabe/astrolabe.sna",
            9,
            3,
            2368131,
            "0a678d2d15b5850e8847bfbfa13e2d22bfae3b4e1df099a0309ae844305bf772",
        ),
        (
            "Fix.rtb",
            6,
            6,
            3946440,
            "7618b06dfda9339b296c74536a2acba11187e0de9f97790afd0681f493fb3484",
        ),
        (
            "Fix.rtp",
            1,
            1,
            6312,
            "a368bb5d7a591d459a93b5d921c8b7a0890d44340b6859326b923fa660d499e2",
        ),
        (
            "Fix.rtt",
            1,
            1,
            3480,
            "417971191c2387fb6e7561b7248c88faf265129e285df4ad4ca6a44a157db85f",
        ),
        (
            "astrolabe/astrolabe.rtb",
            3,
            3,
            559376,
            "235c3fd3b4a713155cd825c231f988913dab65490017945ed6c4563f0d06b42e",
        ),
        (
            "astrolabe/astrolabe.rtp",
            1,
            1,
            688,
            "f5052e206f0706851065d4044c7e6e4af4310c6b32526f6dc231ecc0345ff456",
        ),
        (
            "astrolabe/astrolabe.rtt",
            1,
            1,
            1000,
            "4b8a3c04dcaa358a6d8455687a484d2e086f33f6ccaa1a9f071d2712dff7dcd9",
        ),
        (
            "astrolabe/fixlvl.rtb",
            6,
            4,
            8936,
            "886405725bbd8f4d9ac407c96c8bd0705619788a08f85941994e85910b14b0c1",
        ),
    ];
    let temp = tempfile::tempdir()?;
    let mut manifest = Vec::new();
    let mut raw_digest = Sha256::new();
    let mut total_raw = 0;
    for (name, expected_groups, expected_envelopes, expected_raw, expected_hash) in fixtures {
        let source = fs::read(game.join("Gamedata/World/Levels").join(name))
            .with_context(|| format!("required source fixture {name}"))?;
        assert_eq!(
            format!("{:x}", Sha256::digest(&source)),
            expected_hash,
            "source edition changed: {name}"
        );
        let (mut position, mut groups, mut envelopes, mut raw_bytes) = (0, 0, 0, 0);
        let is_sna = name.ends_with(".sna");
        let target_groups = if is_sna {
            usize::MAX
        } else {
            position = 1;
            source[0] as usize
        };
        while position < source.len() && groups < target_groups {
            // Raymap Montreal framing: module,id,base or count; later engines
            // add a byte before base. Hype does not have that later-engine byte.
            ensure!(
                position + 6 <= source.len(),
                "truncated source block header"
            );
            let (module, id) = (source[position], source[position + 1]);
            let field = word(&source, position + 2);
            position += 6;
            groups += 1;
            let expected_size = if is_sna {
                if field == u32::MAX {
                    continue;
                }
                ensure!(
                    position + 16 <= source.len(),
                    "truncated SNA allocation header"
                );
                let size = word(&source, position + 12) as usize;
                position += 16;
                size
            } else {
                field as usize * 8
            };
            if expected_size == 0 {
                continue;
            }
            let start = position;
            let (raw, consumed) = decode_envelope(&source[start..], DEFAULT_MAX_DECODED)
                .with_context(|| format!("{name}:{module}:{id}"))?;
            assert_eq!(
                raw.len(),
                expected_size,
                "source size/count differs: {name}:{module}:{id}"
            );
            assert_eq!(consumed, 20 + word(&source, start + 4) as usize);
            position += consumed;
            envelopes += 1;
            raw_bytes += raw.len();
            total_raw += raw.len();
            raw_digest.update(&raw);
            let prefix = temp.path().join(manifest.len().to_string());
            let raw_path = prefix.with_extension("raw");
            let stream_path = prefix.with_extension("lzo");
            let envelope_path = prefix.with_extension("envelope");
            let original_path = prefix.with_extension("original");
            fs::write(&raw_path, &raw)?;
            fs::write(&stream_path, compress(&raw)?)?;
            fs::write(&envelope_path, encode_envelope(&raw)?)?;
            fs::write(&original_path, &source[start..position])?;
            manifest.push(json!({"label":format!("{name}:{module}:{id}"),"raw":raw_path,"compressed":stream_path,"envelope":envelope_path,"original_envelope":original_path}));
        }
        assert_eq!(position, source.len());
        assert_eq!(groups, expected_groups);
        assert_eq!(envelopes, expected_envelopes);
        assert_eq!(raw_bytes, expected_raw);
        println!("{name}: {groups} groups, {envelopes} envelopes, {raw_bytes} decoded bytes");
    }
    let manifest_path = temp.path().join("manifest.json");
    fs::write(&manifest_path, serde_json::to_vec(&manifest)?)?;
    let output = Command::new("python3")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tools/check_lzo.py"))
        .arg(manifest_path)
        .output()?;
    ensure!(
        output.status.success(),
        "independent real LZO verification failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(report["cases"].as_u64(), Some(26));
    assert_eq!(report["original_envelopes"].as_u64(), Some(26));
    assert_eq!(report["decoded_bytes"].as_u64(), Some(total_raw as u64));
    let decoded_hash = format!("{:x}", raw_digest.finalize());
    // Independently decoded from the nine pinned source files with liblzo2.
    assert_eq!(
        decoded_hash,
        "b6afb213d53c9398e532c98e207ebea1c4c0110b13e8590337ce364a208fe3b6"
    );
    assert_eq!(
        report["decoded_sha256"].as_str(),
        Some(decoded_hash.as_str())
    );
    println!(
        "independent source/new compression: {}",
        String::from_utf8_lossy(&output.stdout).trim()
    );
    Ok(())
}
