use anyhow::{Context, Result};
use astrolabe::openspace::compression::*;

#[test]
fn adler_and_lzo_edit_roundtrip() -> Result<()> {
    assert_eq!(checksum(b"Wikipedia"), 0x11e60398);
    for data in [
        vec![],
        vec![17],
        b"hello".repeat(3000),
        (0_u8..=255).cycle().take(10000).collect(),
    ] {
        let encoded = compress(&data)?;
        assert_eq!(
            decompress(&encoded, data.len(), DEFAULT_MAX_DECODED).with_context(|| format!(
                "input length {}, encoded prefix {:?}",
                data.len(),
                &encoded[..encoded.len().min(16)]
            ))?,
            data
        );
        let envelope = encode_envelope(&data)?;
        let (decoded, consumed) = decode_envelope(&envelope, DEFAULT_MAX_DECODED)?;
        assert_eq!(consumed, envelope.len());
        assert_eq!(decoded, data);
    }
    Ok(())
}
#[test]
fn rejects_truncation_corruption_and_allocation_bombs() -> Result<()> {
    let mut envelope = encode_envelope(&vec![7; 10000])?;
    assert!(decode_envelope(&envelope, 100).is_err());
    assert!(decode_envelope(&envelope[..envelope.len() - 1], DEFAULT_MAX_DECODED).is_err());
    envelope[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode_envelope(&envelope, DEFAULT_MAX_DECODED).is_err());
    let mut encoded = compress(&[3; 100])?;
    encoded[0] ^= 0xff;
    assert!(decompress(&encoded, 100, DEFAULT_MAX_DECODED).is_err());
    Ok(())
}

#[test]
fn independent_liblzo2_accepts_new_streams_including_empty() -> Result<()> {
    use serde_json::json;
    use std::{fs, process::Command};
    let temp = tempfile::tempdir()?;
    let mut manifest = Vec::new();
    let mut noise = Vec::new();
    let mut state = 0x1234_5678u32;
    for _ in 0..100_000 {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        noise.push(state as u8);
    }
    let cases = [
        vec![],
        vec![0],
        vec![17],
        vec![255; 3],
        vec![0; 256],
        vec![37; 65536],
        b"a block with independently decoded literal and repeated content\n".repeat(700),
        noise,
    ];
    assert_eq!(compress(&[])?, [17, 0, 0]);
    assert_eq!(
        lzokay::compress::compress(&[]).unwrap(),
        [17, 17, 0, 0],
        "revisit fixed-version regression when upgrading lzokay"
    );
    for (index, raw) in cases.iter().enumerate() {
        let raw_path = temp.path().join(format!("{index}.raw"));
        let compressed_path = temp.path().join(format!("{index}.lzo"));
        let envelope_path = temp.path().join(format!("{index}.envelope"));
        fs::write(&raw_path, raw)?;
        fs::write(&compressed_path, compress(raw)?)?;
        fs::write(&envelope_path, encode_envelope(raw)?)?;
        manifest.push(json!({"label":format!("synthetic-{index}"),"raw":raw_path,"compressed":compressed_path,"envelope":envelope_path}));
    }
    let path = temp.path().join("manifest.json");
    fs::write(&path, serde_json::to_vec(&manifest)?)?;
    let output = Command::new("python3")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/tools/check_lzo.py"))
        .arg(path)
        .output()?;
    anyhow::ensure!(
        output.status.success(),
        "independent LZO check failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(report["cases"].as_u64(), Some(cases.len() as u64));
    assert_eq!(report["empty_regression_status"].as_i64(), Some(-6));
    println!(
        "independent synthetic LZO: {}",
        String::from_utf8_lossy(&output.stdout).trim()
    );
    Ok(())
}

#[test]
fn envelope_boundaries_checksums_modes_and_lengths_are_independent() -> Result<()> {
    fn raw_envelope(data: &[u8]) -> Vec<u8> {
        // Known literal framing independently of the production encoder.
        let mut bytes = Vec::new();
        for value in [
            0,
            data.len() as u32,
            checksum(data),
            data.len() as u32,
            checksum(data),
        ] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend(data);
        bytes
    }
    let first = raw_envelope(b"Wikipedia");
    assert_eq!(&first[8..12], &0x11e6_0398u32.to_le_bytes());
    assert_eq!(decode_envelope(&first, 9)?, (b"Wikipedia".to_vec(), 29));
    let second = encode_envelope(&[42; 2048])?;
    let mut joined = first.clone();
    joined.extend(&second);
    let (raw, n) = decode_envelope(&joined, 2048)?;
    assert_eq!(raw, b"Wikipedia");
    assert_eq!(decode_envelope(&joined[n..], 2048)?.0, vec![42; 2048]);
    for offset in [0, 4, 8, 12, 16, 20] {
        let mut bad = first.clone();
        bad[offset] ^= 2;
        assert!(
            decode_envelope(&bad, 1024).is_err(),
            "accepted inconsistent envelope at {offset}"
        );
    }
    let packed = compress(&[7; 300])?;
    assert!(decompress(&packed, 301, 1024).is_err());
    assert!(decompress(&packed, 299, 1024).is_err());
    let mut trailing = packed;
    trailing.push(0);
    assert!(decompress(&trailing, 300, 1024).is_err());
    Ok(())
}
