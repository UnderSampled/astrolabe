#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
use anyhow::{Result, ensure};
use astrolabe_testkit::{instrumentation::Scratch, linux::Owner};
use std::{
    fs,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[test]
fn native_and_i386_scratch_lifetimes_are_bounded() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let fixture = temp.path().join("fixture.json");
    let mut owner = Owner::new()?;
    let process = owner.spawn(
        Command::new(env!("CARGO_BIN_EXE_astrolabe-testkit"))
            .arg("fixture")
            .arg(&fixture)
            .stdout(Stdio::null()),
    )?;
    let deadline = Instant::now() + Duration::from_secs(2);
    while !fixture.exists() {
        ensure!(
            Instant::now() < deadline,
            "fixture readiness deadline expired"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    let started = Instant::now();
    let scratch = Scratch::allocate(&process, 137)?;
    let address = scratch.address();
    scratch.write(99, b"owned native instrumentation")?;
    assert_eq!(scratch.read(99, 28)?, b"owned native instrumentation");
    assert!(scratch.write(4096, b"x").is_err());
    assert!(scratch.read(4096, 1).is_err());
    scratch.close()?;
    ensure!(
        started.elapsed() < Duration::from_secs(4),
        "allocation/close exceeded budget"
    );
    assert!(process.read_running(address, 1).is_err());
    // A pre-existing group stop is rejected without resuming or killing it.
    let pause = process.pause(Duration::from_secs(1))?;
    assert!(Scratch::allocate(&process, 128).is_err());
    assert!(process.check().is_ok());
    drop(pause);
    let mut callback = Scratch::allocate(&process, 4096)?;
    let callback_address = callback.address();
    callback.write(0, &0x12345678u32.to_le_bytes())?;
    callback.write(64, &[0xc3])?;
    assert!(callback.install_callback(callback_address, 0, 64).is_err());
    callback.install_callback(callback_address, 0x12345678, 64)?;
    assert_eq!(
        callback.read(0, 4)?,
        ((callback_address + 64) as u32).to_le_bytes()
    );
    assert!(callback.write(64, &[0xc3]).is_err());
    callback.close()?;
    assert_eq!(
        process.read_running(callback_address, 4)?,
        0x12345678u32.to_le_bytes()
    );
    // Restoration does not free a mapping that a callback may still return to.
    assert_eq!(process.read_running(callback_address + 64, 1)?, [0xc3]);
    owner.close()?;
    drop(owner);

    let churn_ready = temp.path().join("churn-ready");
    let script = "import threading,time,pathlib,sys\ndef churn():\n while True:\n  t=threading.Thread(target=lambda:time.sleep(.002));t.start();t.join()\nfor _ in range(8): threading.Thread(target=churn,daemon=True).start()\npathlib.Path(sys.argv[1]).write_text('ready')\nwhile True: time.sleep(1)\n";
    let mut owner = Owner::new()?;
    let churn = owner.spawn(
        Command::new("python3")
            .args(["-c", script])
            .arg(&churn_ready),
    )?;
    let deadline = Instant::now() + Duration::from_secs(2);
    while !churn_ready.exists() {
        ensure!(
            Instant::now() < deadline,
            "clone churn fixture readiness expired"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    for _ in 0..3 {
        Scratch::allocate(&churn, 4096)?.close()?;
    }
    owner.close()?;
    drop(owner);

    // A genuine ELF32 process, independent of multilib libc/toolchain support.
    // Header plus two program instructions: pause syscall and jump back.
    let binary = temp.path().join("compat-fixture");
    let mut elf = vec![0u8; 84];
    elf[..7].copy_from_slice(b"\x7fELF\x01\x01\x01");
    elf[16..18].copy_from_slice(&2u16.to_le_bytes());
    elf[18..20].copy_from_slice(&3u16.to_le_bytes());
    elf[20..24].copy_from_slice(&1u32.to_le_bytes());
    elf[24..28].copy_from_slice(&0x08048054u32.to_le_bytes());
    elf[28..32].copy_from_slice(&52u32.to_le_bytes());
    elf[40..42].copy_from_slice(&52u16.to_le_bytes());
    elf[42..44].copy_from_slice(&32u16.to_le_bytes());
    elf[44..46].copy_from_slice(&1u16.to_le_bytes());
    elf[52..56].copy_from_slice(&1u32.to_le_bytes());
    elf[60..64].copy_from_slice(&0x08048000u32.to_le_bytes());
    elf[64..68].copy_from_slice(&0x08048000u32.to_le_bytes());
    elf[68..72].copy_from_slice(&93u32.to_le_bytes());
    elf[72..76].copy_from_slice(&93u32.to_le_bytes());
    elf[76..80].copy_from_slice(&5u32.to_le_bytes());
    elf[80..84].copy_from_slice(&4096u32.to_le_bytes());
    elf.extend_from_slice(&[0xb8, 29, 0, 0, 0, 0xcd, 0x80, 0xeb, 0xf7]);
    fs::write(&binary, elf)?;
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o700))?;
    let mut owner = Owner::new()?;
    let process = owner.spawn(&mut Command::new(binary))?;
    std::thread::sleep(Duration::from_millis(30));
    let scratch = Scratch::allocate(&process, 4096)?;
    let address = scratch.address();
    scratch.write(0, b"real i386 mmap2")?;
    assert_eq!(scratch.read(0, 15)?, b"real i386 mmap2");
    scratch.close()?;
    assert!(process.read_running(address, 1).is_err());
    // Failure fallback kills instead of unmapping potentially executing code.
    drop(Scratch::allocate(&process, 4096)?);
    owner.close()?;
    assert!(process.check().is_err());
    Ok(())
}
