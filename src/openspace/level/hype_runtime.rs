//! Provisional build-specific declarations for native Hype instrumentation.
//! These addresses are admitted only after the complete executable hash and
//! live code bytes are verified. They are source-boundary facts, never Rete.
use super::live::{HypeBuild, ReadMemory};
use anyhow::{Context, Result, ensure};

pub const UPDATE_CALLBACK_SLOT: u32 = 0x005e_434c;
pub const UPDATE_CALLBACK: u32 = 0x0044_a1e0;
pub const INPUT_CALLBACK_SLOT: u32 = 0x005e_4344;
pub const INPUT_CALLBACK: u32 = 0x0044_dbd0;
pub const UPDATE_COUNTER: u32 = 0x0075_4aa8;
pub const CLOCK_MS: u32 = 0x0075_4ab0;
pub const RAW_DELTA_MS: u32 = 0x0075_4ab4;
pub const CLAMPED_DELTA_MS: u32 = 0x0075_4af8;
pub const MAIN_ACTOR_ROOT: u32 = 0x0075_6e70;
pub const DEVICE_TABLE: u32 = 0x0087_4250;
pub const DEVICE_STRIDE: u32 = 0x254;
pub const DEVICE_COUNT: usize = 20;
pub const DEVICE_CALLBACK_OFFSET: u32 = 0x1c;
pub const KEYBOARD_KIND: u8 = 8;
pub const KEYBOARD_CALLBACK: u32 = 0x004a_0cd0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyboardDevice {
    pub address: u32,
    pub callback_slot: u32,
}

fn exact(memory: &mut impl ReadMemory, address: u32, size: usize) -> Result<Vec<u8>> {
    let bytes = memory.read(address, size)?;
    ensure!(bytes.len() == size, "truncated runtime read");
    Ok(bytes)
}
fn u32_exact(memory: &mut impl ReadMemory, address: u32) -> Result<u32> {
    let bytes = exact(memory, address, 4)?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

pub fn keyboard_device(_build: &HypeBuild, memory: &mut impl ReadMemory) -> Result<KeyboardDevice> {
    let table = exact(memory, DEVICE_TABLE, DEVICE_COUNT * DEVICE_STRIDE as usize)?;
    let mut matches = vec![];
    for index in 0..DEVICE_COUNT {
        let offset = index * DEVICE_STRIDE as usize;
        let address = u32::from_le_bytes(table[offset..offset + 4].try_into()?);
        if address == 0 {
            continue;
        }
        let header = exact(memory, address, DEVICE_CALLBACK_OFFSET as usize + 4)?;
        if header[0] != KEYBOARD_KIND {
            continue;
        }
        let callback = u32::from_le_bytes(
            header[DEVICE_CALLBACK_OFFSET as usize..DEVICE_CALLBACK_OFFSET as usize + 4]
                .try_into()?,
        );
        ensure!(
            callback == KEYBOARD_CALLBACK,
            "keyboard device has unexpected callback {callback:#010x}"
        );
        matches.push(KeyboardDevice {
            address,
            callback_slot: address
                .checked_add(DEVICE_CALLBACK_OFFSET)
                .context("keyboard callback address overflow")?,
        });
    }
    ensure!(matches.len() == 1, "expected one live keyboard device");
    Ok(matches[0])
}

pub fn verify_callbacks(_build: &HypeBuild, memory: &mut impl ReadMemory) -> Result<()> {
    for (slot, expected, label) in [
        (UPDATE_CALLBACK_SLOT, UPDATE_CALLBACK, "gameplay"),
        (INPUT_CALLBACK_SLOT, INPUT_CALLBACK, "input"),
    ] {
        let actual = u32_exact(memory, slot)?;
        ensure!(
            actual == expected,
            "unexpected {label} callback {actual:#010x}"
        );
    }
    // Verify the caller's post-gameplay order: indirect gameplay, graphics,
    // GLI end, indirect input. This makes a return from the first callback a
    // precise pre-graphics-end/swap publication site for fields whose writer analysis
    // is separately established.
    ensure!(
        exact(memory, 0x0040_4a48, 23)?
            == [
                0xff, 0x15, 0x4c, 0x43, 0x5e, 0x00, 0xff, 0x15, 0x50, 0x43, 0x5e, 0x00, 0xe8, 0xa7,
                0xb8, 0x07, 0x00, 0xff, 0x15, 0x44, 0x43, 0x5e, 0x00,
            ],
        "main callback sequence differs from verified build"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openspace::level::live::HYPE_EXECUTABLE_SHA256;
    use anyhow::{Context, Result};
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct Memory(BTreeMap<u32, Vec<u8>>);
    impl Memory {
        fn put(&mut self, address: u32, bytes: impl Into<Vec<u8>>) {
            self.0.insert(address, bytes.into());
        }
    }
    impl ReadMemory for Memory {
        fn read(&mut self, address: u32, size: usize) -> Result<Vec<u8>> {
            for (&start, bytes) in self.0.range(..=address).rev() {
                let offset = (address - start) as usize;
                if offset
                    .checked_add(size)
                    .is_some_and(|end| end <= bytes.len())
                {
                    return Ok(bytes[offset..offset + size].to_vec());
                }
            }
            Err(anyhow::anyhow!("missing memory at {address:#x}")).context("fake runtime")
        }
    }
    fn profile() -> Memory {
        let mut memory = Memory::default();
        memory.put(UPDATE_CALLBACK_SLOT, UPDATE_CALLBACK.to_le_bytes());
        memory.put(INPUT_CALLBACK_SLOT, INPUT_CALLBACK.to_le_bytes());
        memory.put(
            0x0040_4a48,
            vec![
                0xff, 0x15, 0x4c, 0x43, 0x5e, 0x00, 0xff, 0x15, 0x50, 0x43, 0x5e, 0x00, 0xe8, 0xa7,
                0xb8, 0x07, 0x00, 0xff, 0x15, 0x44, 0x43, 0x5e, 0x00,
            ],
        );
        let mut table = vec![0; DEVICE_COUNT * DEVICE_STRIDE as usize];
        table[3 * DEVICE_STRIDE as usize..3 * DEVICE_STRIDE as usize + 4]
            .copy_from_slice(&0x031b_11a4_u32.to_le_bytes());
        memory.put(DEVICE_TABLE, table);
        let mut device = vec![0; DEVICE_CALLBACK_OFFSET as usize + 4];
        device[0] = KEYBOARD_KIND;
        device[DEVICE_CALLBACK_OFFSET as usize..][..4]
            .copy_from_slice(&KEYBOARD_CALLBACK.to_le_bytes());
        memory.put(0x031b_11a4, device);
        memory
    }
    #[test]
    fn exact_profile_and_dynamic_keyboard_are_both_required() -> Result<()> {
        let build = HypeBuild::verified(HYPE_EXECUTABLE_SHA256)?;
        let mut memory = profile();
        verify_callbacks(&build, &mut memory)?;
        assert_eq!(
            keyboard_device(&build, &mut memory)?,
            KeyboardDevice {
                address: 0x031b_11a4,
                callback_slot: 0x031b_11c0,
            }
        );
        memory.0.get_mut(&UPDATE_CALLBACK_SLOT).unwrap()[0] ^= 1;
        assert!(verify_callbacks(&build, &mut memory).is_err());
        Ok(())
    }

    #[test]
    fn malformed_runtime_profile_is_rejected_without_panics() -> Result<()> {
        let build = HypeBuild::verified(HYPE_EXECUTABLE_SHA256)?;
        let mut short = profile();
        short.0.get_mut(&DEVICE_TABLE).unwrap().truncate(7);
        assert!(keyboard_device(&build, &mut short).is_err());

        let mut duplicate = profile();
        let table = duplicate.0.get_mut(&DEVICE_TABLE).unwrap();
        table[7 * DEVICE_STRIDE as usize..7 * DEVICE_STRIDE as usize + 4]
            .copy_from_slice(&0x031b_11a4_u32.to_le_bytes());
        assert!(keyboard_device(&build, &mut duplicate).is_err());

        let mut callback = profile();
        callback.0.get_mut(&0x031b_11a4).unwrap()[DEVICE_CALLBACK_OFFSET as usize] ^= 1;
        assert!(keyboard_device(&build, &mut callback).is_err());

        let mut overflow = profile();
        overflow.0.get_mut(&DEVICE_TABLE).unwrap()[3 * DEVICE_STRIDE as usize..][..4]
            .copy_from_slice(&u32::MAX.to_le_bytes());
        let mut device = vec![0; DEVICE_CALLBACK_OFFSET as usize + 4];
        device[0] = KEYBOARD_KIND;
        device[DEVICE_CALLBACK_OFFSET as usize..][..4]
            .copy_from_slice(&KEYBOARD_CALLBACK.to_le_bytes());
        overflow.put(u32::MAX, device);
        assert!(keyboard_device(&build, &mut overflow).is_err());
        Ok(())
    }
}
