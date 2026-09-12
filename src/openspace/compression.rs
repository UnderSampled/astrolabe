//! Hype's 20-byte compression envelope and bounded LZO1X payloads.
//! Evidence: Raymap SNA.ReadSNABlock and RelocationTable.Read;
//! checksums corroborated against actual Hype files and independent zlib Adler32.
use anyhow::{Context, Result, ensure};
use binrw::{BinRead, BinWrite};
use std::io::Cursor;

pub const DEFAULT_MAX_DECODED: usize = 64 * 1024 * 1024;

#[derive(Debug, BinRead, BinWrite)]
#[brw(little)]
struct Envelope {
    compressed: u32,
    stored_size: u32,
    stored_checksum: u32,
    decoded_size: u32,
    decoded_checksum: u32,
}

/// Adler32 as used by the Hype engine. Chunking bounds the arithmetic.
pub fn checksum(bytes: &[u8]) -> u32 {
    let (mut a, mut b) = (1_u32, 0_u32);
    for chunk in bytes.chunks(5552) {
        for &byte in chunk {
            a += byte as u32;
            b += a;
        }
        a %= 65521;
        b %= 65521;
    }
    a | (b << 16)
}

pub fn decompress(bytes: &[u8], expected: usize, limit: usize) -> Result<Vec<u8>> {
    ensure!(
        expected <= limit,
        "LZO decoded size {expected} exceeds limit {limit}"
    );
    let mut output = vec![0; expected];
    let actual = lzokay::decompress::decompress(bytes, &mut output).context("LZO1X decode")?;
    ensure!(
        actual == expected,
        "LZO decoded length {actual} differs from declared {expected}"
    );
    Ok(output)
}

pub fn compress(bytes: &[u8]) -> Result<Vec<u8>> {
    // LZO1X empty stream is just its terminator. lzokay 2.0.1 emits an extra
    // literal marker for empty input; the independent liblzo2 check rejects it.
    if bytes.is_empty() {
        return Ok(vec![0x11, 0, 0]);
    }
    lzokay::compress::compress(bytes).context("LZO1X encode")
}

/// Decode one envelope, returning content and consumed input length.
/// The caller can then continue to the next block without copying source layout into the model.
pub fn decode_envelope(bytes: &[u8], limit: usize) -> Result<(Vec<u8>, usize)> {
    let header = Envelope::read(&mut Cursor::new(bytes)).context("Hype compression header")?;
    ensure!(
        header.compressed <= 1,
        "unsupported Hype compression mode {}",
        header.compressed
    );
    ensure!(
        header.decoded_size as usize <= limit,
        "Hype decoded size exceeds limit {limit}"
    );
    let end = 20_usize
        .checked_add(header.stored_size as usize)
        .context("stored size overflow")?;
    let stored = bytes.get(20..end).context("truncated Hype payload")?;
    ensure!(
        checksum(stored) == header.stored_checksum,
        "stored Hype checksum mismatch"
    );
    let output = if header.compressed == 1 {
        decompress(stored, header.decoded_size as usize, limit)?
    } else {
        ensure!(
            header.stored_size == header.decoded_size,
            "uncompressed envelope lengths differ"
        );
        stored.to_vec()
    };
    ensure!(
        checksum(&output) == header.decoded_checksum,
        "decoded Hype checksum mismatch"
    );
    Ok((output, end))
}

/// Encoding decisions, lengths, and checksums are derived from the current content.
pub fn encode_envelope(bytes: &[u8]) -> Result<Vec<u8>> {
    let encoded = compress(bytes)?;
    let (compressed, stored) = if encoded.len() < bytes.len() {
        (1, encoded.as_slice())
    } else {
        (0, bytes)
    };
    let header = Envelope {
        compressed,
        stored_size: stored
            .len()
            .try_into()
            .context("stored payload over 4GiB")?,
        stored_checksum: checksum(stored),
        decoded_size: bytes
            .len()
            .try_into()
            .context("decoded payload over 4GiB")?,
        decoded_checksum: checksum(bytes),
    };
    let mut writer = Cursor::new(Vec::with_capacity(20 + stored.len()));
    header.write(&mut writer)?;
    let mut result = writer.into_inner();
    result.extend_from_slice(stored);
    Ok(result)
}
