//! Native prepared-fixture inspection. The outer process owns Gamescope and a
//! bounded worker; that worker links Astrolabe directly and owns Wine. The
//! Python controller remains the migration reference for OS keyboard semantics.
use crate::{
    config::{Config, digest},
    desktop::{Desktop, SetupKey},
    instrumentation::Scratch,
    linux::{Identity, Owner, Process},
    replay::{Edge, Key, boundary_recording},
};
use anyhow::{Context, Result, ensure};
use astrolabe::openspace::level::hype_runtime;
use astrolabe::openspace::level::live::{HypeBuild, ReadMemory};
use serde_json::json;
use std::{
    fs::{self, File},
    os::{fd::AsRawFd, unix::ffi::OsStrExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

struct Cancellation {
    flag: Arc<AtomicBool>,
    registrations: Vec<signal_hook::SigId>,
}
impl Cancellation {
    fn new() -> Result<Self> {
        let mut result = Self {
            flag: Arc::new(AtomicBool::new(false)),
            registrations: vec![],
        };
        for signal in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP] {
            result
                .registrations
                .push(signal_hook::flag::register(signal, result.flag.clone())?);
        }
        Ok(result)
    }
    fn check(&self) -> Result<()> {
        ensure!(
            !self.flag.load(Ordering::SeqCst),
            "native controller interrupted"
        );
        Ok(())
    }
    fn wait(&self, duration: Duration) -> Result<()> {
        let deadline = Instant::now() + duration;
        while Instant::now() < deadline {
            self.check()?;
            thread::sleep(
                Duration::from_millis(5).min(deadline.saturating_duration_since(Instant::now())),
            );
        }
        self.check()
    }
}
impl Drop for Cancellation {
    fn drop(&mut self) {
        for id in &self.registrations {
            signal_hook::low_level::unregister(*id);
        }
    }
}
fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    use std::io::Write;
    ensure!(
        !path.exists(),
        "evidence file already exists: {}",
        path.display()
    );
    let temporary = path.with_extension("json.tmp");
    let mut file = File::create_new(&temporary)?;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.write_all(b"\n")?;
    drop(file);
    fs::rename(temporary, path)?;
    Ok(())
}
struct Running<'a>(&'a Process);
impl ReadMemory for Running<'_> {
    fn read(&mut self, address: u32, size: usize) -> Result<Vec<u8>> {
        self.0.read_running(address as usize, size)
    }
}

const OBSERVATION_CODE: usize = 0;
const STATE: usize = 512;
const RECORDS: usize = 1024;
const RECORD_SIZE: usize = 44;
const RECORD_COUNT: usize = 18;
const ACTOR_ROOT_ID: u8 = 60;
const IDLE_ACTION: u32 = 47;
const LEFT_ACTION: u32 = 35;
const SHIFT_LEFT_ACTION: u32 = 348;

fn live_locator(value: &str) -> Result<u32> {
    let value = value
        .strip_prefix("live:")
        .context("runtime action member is not a live locator")?;
    u32::from_str_radix(value, 16).context("runtime action member has an invalid address")
}

/// Resolve completed action pointers through the actor's own ordered family.
/// This qualifies the exact physical tape outcomes instead of treating three
/// merely distinct pointers as semantic action identities.
fn qualify_boundary_consumption(
    action_members: &[String],
    observations: &[(u32, u8, u32, u32)],
) -> Result<Vec<u32>> {
    ensure!(
        observations.len() == RECORD_COUNT,
        "boundary consumer observation count differs"
    );
    let members = action_members
        .iter()
        .map(|member| live_locator(member))
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        members.len() > SHIFT_LEFT_ACTION as usize,
        "runtime family omits a boundary action"
    );
    ensure!(
        members
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            == members.len(),
        "runtime family contains duplicate action members"
    );
    observations
        .iter()
        .enumerate()
        .map(|(index, &(ordinal, mask, pointer, counter))| {
            ensure!(
                ordinal as usize == index + 1,
                "boundary consumer observation ordinal differs"
            );
            let action: u32 = members
                .iter()
                .position(|member| *member == pointer)
                .context("completed action pointer is outside the actor family")?
                .try_into()?;
            let expected = match mask {
                0 => IDLE_ACTION,
                1 => LEFT_ACTION,
                3 => SHIFT_LEFT_ACTION,
                _ => anyhow::bail!("unsupported boundary input mask {mask}"),
            };
            ensure!(
                action == expected,
                "physical input was not consumed at ordinal {ordinal}: mask {mask}, action {action}, expected {expected}"
            );
            ensure!(
                ((counter as i32) > 0) == (mask & 1 != 0) && counter as i32 != 0,
                "Left consumer counter disagrees at ordinal {ordinal}"
            );
            Ok(action)
        })
        .collect()
}

fn imm(code: &mut Vec<u8>, value: u32) {
    code.extend_from_slice(&value.to_le_bytes());
}
fn rel8(code: &mut Vec<u8>, opcode: u8) -> usize {
    code.extend_from_slice(&[opcode, 0]);
    code.len() - 1
}
fn rel32(code: &mut Vec<u8>, condition: u8) -> usize {
    code.extend_from_slice(&[0x0f, condition, 0, 0, 0, 0]);
    code.len() - 4
}
fn patch_rel8(code: &mut [u8], site: usize, target: usize) -> Result<()> {
    let displacement = isize::try_from(target)? - isize::try_from(site + 1)?;
    let displacement: i8 = displacement
        .try_into()
        .with_context(|| format!("probe branch outside signed-byte range: {site}->{target}"))?;
    code[site] = displacement as u8;
    Ok(())
}
fn patch_rel32(code: &mut [u8], site: usize, target: usize) -> Result<()> {
    let displacement: i32 = (target as i128 - (site + 4) as i128)
        .try_into()
        .context("probe near branch outside signed-word range")?;
    code[site..site + 4].copy_from_slice(&displacement.to_le_bytes());
    Ok(())
}
/// Diagnostic publication probe. It observes completed gameplay callbacks and
/// never participates in input delivery or patches the keyboard callback.
fn observation_probe_image(base: u32, left_action_counter: u32) -> Result<Vec<u8>> {
    let state = base
        .checked_add(STATE as u32)
        .context("probe state overflow")?;
    let records = base
        .checked_add(RECORDS as u32)
        .context("probe records overflow")?;
    let mut image = vec![0; 4096];
    let mut update = vec![0xb8];
    imm(&mut update, hype_runtime::UPDATE_CALLBACK);
    update.extend_from_slice(&[0xff, 0xd0, 0x9c, 0x60, 0xff, 0x15]);
    imm(&mut update, 0x005c_0810);
    update.extend_from_slice(&[0x8b, 0xe8, 0xbf]);
    imm(&mut update, state);
    update.extend_from_slice(&[0x80, 0x3d]);
    imm(&mut update, hype_runtime::STATUS);
    update.push(9);
    let error_phase = rel32(&mut update, 0x85);
    update.extend_from_slice(&[0x8b, 0x4f, 0x08, 0x85, 0xc9]);
    let done_zero = rel32(&mut update, 0x84);
    update.extend_from_slice(&[0x3b, 0x4f, 0x0c]);
    let done_same = rel32(&mut update, 0x84);
    // Capture each completed callback until the fixed observation limit. Input
    // delivery is independent and never arms this probe.
    update.extend_from_slice(&[0x8b, 0x47, 0x0c, 0x40, 0x89, 0xc1]);
    update.extend_from_slice(&[0x83, 0xf9, RECORD_COUNT as u8]);
    let error_over = rel32(&mut update, 0x87);
    update.extend_from_slice(&[0x8b, 0x57, 0x0c, 0x6b, 0xd2, RECORD_SIZE as u8, 0x81, 0xc2]);
    imm(&mut update, records);
    update.extend_from_slice(&[0x89, 0x0a]);
    for (offset, address) in [
        (4_u8, hype_runtime::UPDATE_COUNTER),
        (8, hype_runtime::CLOCK_MS),
        (12, hype_runtime::RAW_DELTA_MS),
        (16, hype_runtime::CLAMPED_DELTA_MS),
    ] {
        update.push(0xa1);
        imm(&mut update, address);
        update.extend_from_slice(&[0x89, 0x42, offset]);
    }
    update.extend_from_slice(&[0x89, 0x6a, 0x14]);
    update.push(0xa1);
    imm(&mut update, hype_runtime::MAIN_ACTOR_ROOT);
    // MainActor entry -> superobject node -> perso -> data3d -> current action.
    // Every pointer that will be dereferenced is checked before use. A null
    // current-action member is valid optional content and is recorded as zero;
    // the data3d object itself is not optional for this actor contract.
    update.extend_from_slice(&[0x85, 0xc0]);
    let error_root = rel32(&mut update, 0x84);
    // Retain the first accepted actor root as the session identity. A changed
    // root means replacement/reload and aborts before traversing its pointers.
    update.extend_from_slice(&[0x83, 0x7f, ACTOR_ROOT_ID, 0x00]);
    let root_seen = rel8(&mut update, 0x75);
    update.extend_from_slice(&[0x89, 0x47, ACTOR_ROOT_ID]);
    let root_after_init = rel8(&mut update, 0xeb);
    let root_compare = update.len();
    update.extend_from_slice(&[0x3b, 0x47, ACTOR_ROOT_ID]);
    let root_changed = rel32(&mut update, 0x85);
    let root_after = update.len();
    update.extend_from_slice(&[0x89, 0x42, 0x18, 0x8b, 0x40, 0x04, 0x85, 0xc0]);
    let error_node = rel32(&mut update, 0x84);
    update.extend_from_slice(&[0x89, 0x42, 0x1c, 0x8b, 0x40, 0x04, 0x85, 0xc0]);
    let error_perso = rel32(&mut update, 0x84);
    update.extend_from_slice(&[0x8b, 0x00, 0x85, 0xc0]);
    let error_data3d = rel32(&mut update, 0x84);
    update.extend_from_slice(&[
        0x89, 0x42, 0x20, 0x8b, 0x40, 0x04, 0x89, 0x42, 0x24, 0x89, 0x4f, 0x0c,
    ]);
    update.push(0xa1);
    imm(&mut update, left_action_counter);
    update.extend_from_slice(&[0x89, 0x42, 0x28]);
    let done = update.len();
    update.extend_from_slice(&[0x61, 0x9d, 0xc3]);
    let error = update.len();
    update.extend_from_slice(&[0xc7, 0x47, 0x10]);
    imm(&mut update, 1);
    let back = rel8(&mut update, 0xeb);
    patch_rel32(&mut update, done_zero, done)?;
    patch_rel32(&mut update, done_same, done)?;
    patch_rel32(&mut update, error_over, error)?;
    patch_rel32(&mut update, error_phase, error)?;
    patch_rel32(&mut update, error_root, error)?;
    patch_rel8(&mut update, root_seen, root_compare)?;
    patch_rel8(&mut update, root_after_init, root_after)?;
    patch_rel32(&mut update, root_changed, error)?;
    patch_rel32(&mut update, error_node, error)?;
    patch_rel32(&mut update, error_perso, error)?;
    patch_rel32(&mut update, error_data3d, error)?;
    patch_rel8(&mut update, back, done)?;
    ensure!(
        update.len() <= STATE - OBSERVATION_CODE,
        "update wrapper too large"
    );
    image[OBSERVATION_CODE..OBSERVATION_CODE + update.len()].copy_from_slice(&update);
    image[STATE + 8..STATE + 12].copy_from_slice(&(RECORD_COUNT as u32).to_le_bytes());
    Ok(image)
}

fn phase(
    process: &Process,
    build: &HypeBuild,
    wanted: u8,
    level: Option<&str>,
    limit: Duration,
    cancel: &Cancellation,
) -> Result<()> {
    let deadline = Instant::now() + limit;
    loop {
        let status = build.status(&mut Running(process))?;
        if status.phase == wanted && level.is_none_or(|s| s == status.level) {
            return Ok(());
        }
        ensure!(
            Instant::now() < deadline,
            "game did not reach phase {wanted} / {level:?}; observed {} / {}",
            status.phase,
            status.level
        );
        cancel.wait(Duration::from_millis(50))?;
    }
}
fn find_game(owner: &mut Owner, config: &Config, cancel: &Cancellation) -> Result<Process> {
    let deadline = Instant::now() + Duration::from_secs(40);
    loop {
        let mut found = vec![];
        for process in owner.processes()? {
            let pid = process.identity().pid;
            let cmd = fs::read(format!("/proc/{pid}/cmdline")).unwrap_or_default();
            if cmd.split(|b| *b == 0).next() != Some(config.exe.as_os_str().as_bytes()) {
                continue;
            }
            let environment = fs::read(format!("/proc/{pid}/environ")).unwrap_or_default();
            let prefix = format!("WINEPREFIX={}", config.prefix.display());
            if !environment
                .split(|b| *b == 0)
                .any(|v| v == prefix.as_bytes())
            {
                continue;
            }
            if process.read_running(0x400000, 2).is_ok_and(|b| b == b"MZ") {
                found.push(process);
            }
        }
        ensure!(found.len() <= 1, "ambiguous owned game processes");
        if let Some(process) = found.pop() {
            return Ok(process);
        }
        ensure!(
            Instant::now() < deadline,
            "Wine game did not become memory-readable"
        );
        cancel.wait(Duration::from_millis(50))?;
    }
}

/// Only the Gamescope-launched worker calls this. Its DISPLAY/XAUTHORITY are
/// inherited at exec from the isolated server, never changed in a running process.
pub fn worker(
    config_path: &Path,
    output: &Path,
    host_display: &str,
    fail_stopped_worker: bool,
    replay: bool,
) -> Result<()> {
    let cancel = Cancellation::new()?;
    let config = Config::load(config_path)?;
    let build = HypeBuild::verified(&config.exe_sha256)?;
    let started = Instant::now();
    let mut owner = Owner::new()?;
    write_json(&output.join("worker-started.json"), &Identity::current()?)?;
    let result = (|| -> Result<serde_json::Value> {
        ensure!(
            std::env::var("DISPLAY").is_ok_and(|d| d != host_display && d.starts_with(':')),
            "worker is not on a private display"
        );
        let wine_log = File::create_new(output.join("wine.log"))?;
        let mut command = Command::new(&config.wine);
        command
            .arg(&config.exe)
            .current_dir(&config.game)
            .envs(config.environment()?)
            .stdout(Stdio::from(wine_log.try_clone()?))
            .stderr(Stdio::from(wine_log));
        owner.spawn(&mut command)?;
        let process = find_game(&mut owner, &config, &cancel)?;
        // Verify the mapped image header against the fully hashed local module.
        let image = fs::read(&config.exe)?;
        ensure!(
            process.read_running(0x400000, 512)?
                == image.get(..512).context("short executable header")?,
            "live image header differs from pinned module"
        );
        phase(&process, &build, 38, None, Duration::from_secs(40), &cancel)?;
        let mut desktop = Desktop::connect(host_display, process.identity().pid as u32)?;
        cancel.wait(Duration::from_millis(500))?;
        desktop.focus()?;
        cancel.wait(Duration::from_millis(800))?;
        write_json(
            &output.join("menu-ready.json"),
            &json!({"phase":38,"pid":process.identity(),"display":desktop.display}),
        )?;
        if fail_stopped_worker {
            let _stopped = process.pause(Duration::from_secs(1))?;
            write_json(
                &output.join("fault-armed.json"),
                &json!({"fault":"worker death while game is stopped","process":process.identity()}),
            )?;
            // SAFETY: deliberate acceptance-test worker death, without unwinding
            // or resuming the game. The separate native owner must clean it up.
            unsafe {
                libc::_exit(77);
            }
        }
        let mut setup = vec![];
        for key in [SetupKey::Down, SetupKey::Return, SetupKey::Return] {
            let before = started.elapsed().as_micros();
            desktop.setup_key(key, |duration| cancel.wait(duration))?;
            setup.push(json!({"key":key,"start_micros":before,"end_micros":started.elapsed().as_micros(),"purpose":"load supplied save; timed menu setup only"}));
        }
        phase(
            &process,
            &build,
            9,
            Some("manoir"),
            Duration::from_secs(15),
            &cancel,
        )?;
        cancel.wait(Duration::from_secs(4))?;
        if replay {
            let prepared = boundary_recording()?;
            let recording = prepared.recording();
            ensure!(
                recording.updates.len() == RECORD_COUNT,
                "unexpected replay length"
            );
            let mut running = Running(&process);
            hype_runtime::verify_callbacks(&build, &mut running)?;
            let keyboard = hype_runtime::keyboard_device(&build, &mut running)?;
            // Match the proven Python controller: focus the owned Wine window,
            // allow the guest focus event to settle, then send literal XTest
            // edges. The keyboard callback and device buffer stay untouched.
            desktop.focus()?;
            cancel.wait(Duration::from_millis(200))?;
            let replay_started = Instant::now();
            let mut dispatch_times = vec![None; RECORD_COUNT];
            ensure!(
                recording.updates[0].events.is_empty(),
                "initial OS-input update must be neutral for this fixture"
            );
            dispatch_times[0] = Some(replay_started.elapsed().as_micros());

            let mut scratch = Scratch::allocate(&process, 4096)?;
            let image = observation_probe_image(
                scratch.address() as u32,
                hype_runtime::LEFT_ACTION_COUNTER,
            )?;
            scratch.write(0, &image)?;
            scratch.install_callback(
                hype_runtime::UPDATE_CALLBACK_SLOT as usize,
                hype_runtime::UPDATE_CALLBACK,
                OBSERVATION_CODE,
            )?;
            let mut last_completed = 0_u32;
            loop {
                let header = scratch.read(STATE + 8, 12)?;
                let completed = u32::from_le_bytes(header[4..8].try_into()?);
                let error = u32::from_le_bytes(header[8..12].try_into()?);
                ensure!(
                    error == 0,
                    "native replay hook reported ordinal failure {error}"
                );
                if completed == RECORD_COUNT as u32 {
                    break;
                }
                if completed != last_completed {
                    ensure!(
                        completed == last_completed + 1,
                        "OS input dispatcher missed completed-update boundary: {last_completed}->{completed}"
                    );
                    let next = completed as usize;
                    if !recording.updates[next].events.is_empty() {
                        desktop.physical_events(&recording.updates[next].events)?;
                    }
                    dispatch_times[next] = Some(replay_started.elapsed().as_micros());
                    let after = scratch.read(STATE + 12, 4)?;
                    ensure!(
                        u32::from_le_bytes(after.as_slice().try_into()?) == completed,
                        "OS input edge arrived after its intended completed-update boundary"
                    );
                    last_completed = completed;
                }
                ensure!(
                    replay_started.elapsed() < Duration::from_secs(10),
                    "native replay deadline exceeded"
                );
                cancel.wait(Duration::from_millis(1))?;
            }
            let stopped = process.pause(Duration::from_secs(1))?;
            let raw = scratch.read(STATE, RECORDS + RECORD_COUNT * RECORD_SIZE - STATE)?;
            let mut capture = stopped.capture()?;
            let status = build.status(&mut capture)?;
            let actor = build.actor_with_actions(&mut capture, &[])?;
            let _snapshot = capture.finish()?;
            let final_limit = u32::from_le_bytes(raw[8..12].try_into()?);
            let final_completed = u32::from_le_bytes(raw[12..16].try_into()?);
            let final_error = u32::from_le_bytes(raw[16..20].try_into()?);
            ensure!(
                final_limit == RECORD_COUNT as u32
                    && final_completed == RECORD_COUNT as u32
                    && final_error == 0,
                "stopped observation header is inconsistent: limit={final_limit}, completed={final_completed}, error={final_error}"
            );
            drop(stopped);
            ensure!(
                status.phase == 9 && status.level == "manoir",
                "replay epoch changed"
            );
            let mut expected_held = std::collections::BTreeSet::new();
            let mut completed = Vec::with_capacity(RECORD_COUNT);
            for (index, input_update) in recording.updates.iter().enumerate() {
                for event in &input_update.events {
                    match event.edge {
                        Edge::Press => {
                            expected_held.insert(event.key);
                        }
                        Edge::Release => {
                            expected_held.remove(&event.key);
                        }
                    }
                }
                let offset = RECORDS - STATE + index * RECORD_SIZE;
                let row = &raw[offset..offset + RECORD_SIZE];
                let word = |at: usize| u32::from_le_bytes(row[at..at + 4].try_into().unwrap());
                let mask = u8::from(expected_held.contains(&Key::ArrowLeft))
                    | (u8::from(expected_held.contains(&Key::LeftShift)) << 1);
                completed.push((word(0), mask, word(36), word(40)));
            }
            let action_ids = qualify_boundary_consumption(&actor.action_members, &completed)?;
            let mut rows = vec![];
            let mut previous_counter = None;
            let mut actor_identity = None;
            let mut held = std::collections::BTreeSet::new();
            let mut left_counters = vec![];
            let mut action_pointers = vec![];
            for (index, (input_update, dispatch_time)) in
                recording.updates.iter().zip(&dispatch_times).enumerate()
            {
                for event in &input_update.events {
                    match event.edge {
                        Edge::Press => {
                            held.insert(event.key);
                        }
                        Edge::Release => {
                            held.remove(&event.key);
                        }
                    }
                }
                let offset = RECORDS - STATE + index * RECORD_SIZE;
                let row = &raw[offset..offset + RECORD_SIZE];
                let word = |at: usize| u32::from_le_bytes(row[at..at + 4].try_into().unwrap());
                ensure!(
                    word(0) == index as u32 + 1,
                    "missing completed-update ordinal"
                );
                if let Some(previous) = previous_counter {
                    ensure!(
                        word(4) == previous + 1,
                        "non-consecutive engine update counter"
                    );
                }
                previous_counter = Some(word(4));
                let identity = (word(24), word(28), word(32));
                ensure!(
                    actor_identity.is_none_or(|expected| expected == identity),
                    "controlled actor identity changed during replay"
                );
                actor_identity = Some(identity);
                ensure!(
                    word(28) == live_locator(&actor.node)?
                        && word(32) == live_locator(&actor.data3d)?,
                    "completed observation belongs to a different final actor"
                );
                ensure!(
                    (16..=80).contains(&word(16)),
                    "clamped delta outside verified range"
                );
                let action_pointer = word(36);
                let left_counter = word(40) as i32;
                action_pointers.push(action_pointer);
                left_counters.push(left_counter);
                rows.push(json!({"ordinal":word(0),"observation_ordinal":word(0),
                    "update_counter":word(4),"clock_ms":word(8),
                    "raw_delta_ms":word(12),"clamped_delta_ms":word(16),"boundary_time_ms":word(20),
                    "os_dispatch_micros":dispatch_time,
                    "os_events":input_update.events,
                    "planned_left":held.contains(&Key::ArrowLeft),
                    "planned_left_shift":held.contains(&Key::LeftShift),
                    "actor_entry":format!("live:{:08x}",word(24)),"actor_node":format!("live:{:08x}",word(28)),
                    "data3d":format!("live:{:08x}",word(32)),
                    "current_action_pointer":format!("live:{action_pointer:08x}"),
                    "current_action_id":action_ids[index],
                    "left_action_counter":left_counter}));
            }
            ensure!(
                left_counters[0] < 0
                    && left_counters[1] < left_counters[0]
                    && left_counters[2..7] == [1, -1, 1, -1, -2]
                    && left_counters[7..12] == [1, 2, 3, 4, 5]
                    && left_counters[12..18] == [-1, -2, -3, -4, -5, -6],
                "Hype did not consume the OS Left edges at the recorded update ordinals: {left_counters:?}"
            );
            let neutral_action = action_pointers[1];
            let turn_action = action_pointers[2];
            let chord_action = action_pointers[7];
            ensure!(
                turn_action != neutral_action
                    && chord_action != neutral_action
                    && chord_action != turn_action
                    && action_pointers[3] == neutral_action
                    && action_pointers[4] == turn_action
                    && action_pointers[5..7].iter().all(|p| *p == neutral_action)
                    && action_pointers[7..12].iter().all(|p| *p == chord_action)
                    && action_pointers[12..18].iter().all(|p| *p == neutral_action),
                "Hype action transitions did not follow the OS key recording: {action_pointers:?}"
            );
            scratch.close()?;
            desktop.release()?;
            return Ok(
                json!({"schema":1,"status":"replay_complete","scope":"native OS-input delivery with separate completed-update observation",
                "fixture":config,"process":process.identity(),"game":status,"recording_sha256":boundary_recording()?.sha256(),
                "keyboard":{"device":format!("live:{:08x}",keyboard.address),"callback_slot":format!("live:{:08x}",keyboard.callback_slot)},
                "updates":rows,"input_delivery":"x11_xtest_os_events","keyboard_callback_patched":false,
                "keyboard_buffer_written":false,"observation_probe":"completed_update_diagnostic_only",
                "consistency":"completed_update_hook","completed_update":true,"frame_replay":true,
                "consumption_verified":true,"physical_consumption_verified":true,
                "consumer_evidence":{"left_counter_pattern":[1,-1,1,-1,-2,1,2,3,4,5,-1,-2,-3,-4,-5,-6],
                    "current_action_transitions":"neutral -> turn -> neutral -> turn -> neutral -> shifted-left chord -> neutral",
                    "consumer_profile":{"idle_action":IDLE_ACTION,"left_action":LEFT_ACTION,"shift_left_action":SHIFT_LEFT_ACTION}},
                "publication":{"qualified_completed_fields":["current_action_pointer","current_action_id"],
                    "provenance_fields":["update_counter","clock_ms","raw_delta_ms","clamped_delta_ms","boundary_time_ms"],
                    "planned_input_fields":["planned_left","planned_left_shift"],
                    "consumer_observation_fields":["left_action_counter"],
                    "input_contract":"literal OS key edges are dispatched after completed update k for Hype's ordinary keyboard poll and update k+1; the actor's root-resolved family qualifies exact idle, Left and Shift+Left action ids while the independent Left counter verifies consumption and the game owns focus, translation, callback order and bindings",
                    "source_evidence":{"getter":"004424f0","setter":"00442500",
                        "ordinary_update":"004499b0 -> 00433e90 -> 004442e0",
                        "post_gameplay_tail":"0044a670 -> GLI_vEndOfTurnEngine -> 0044dbd0; no current-action setter"}},
                "behavior_parity":false,"elapsed_micros":replay_started.elapsed().as_micros()}),
            );
        }
        let capture_start = started.elapsed().as_micros();
        let stopped = process.pause(Duration::from_secs(1))?;
        let mut capture = stopped.capture()?;
        let status = build.status(&mut capture)?;
        let actor = build.actor(&mut capture)?;
        let snapshot = capture.finish()?;
        drop(stopped);
        let capture_end = started.elapsed().as_micros();
        write_json(&output.join("snapshot.json"), &snapshot)?;
        let screenshot = desktop.screenshot(&output.join("original.png"), started)?;
        desktop.release()?;
        let mut graphics = std::collections::BTreeMap::new();
        for name in [
            "glide3x.dll",
            "ddraw.dll",
            "d3d11.dll",
            "dxgi.dll",
            "dgVoodoo.conf",
            "dxvk.conf",
        ] {
            let path = config.game.join(name);
            if path.is_file() {
                graphics.insert(name, digest(&path)?);
            }
        }
        Ok(
            json!({"schema":1,"status":"inspection_complete","scope":"native fixture launch and stopped-byte actor inspection",
            "fixture":config,"graphics_sha256":graphics,"process":process.identity(),"game":status,"actor":actor,
            "setup":setup,"capture_start_micros":capture_start,"capture_end_micros":capture_end,
            "snapshot":"snapshot.json","screenshot":screenshot,"consistency":"stopped_bytes",
            "completed_update":false,"frame_replay":false,"behavior_parity":false}),
        )
    })();
    let cleanup_started = Instant::now();
    let cleanup = owner.close();
    let mut report = match &result {
        Ok(value) => value.clone(),
        Err(error) => json!({"schema":1,"status":"failed","error":format!("{error:#}")}),
    };
    report["cleanup_seconds"] = json!(cleanup_started.elapsed().as_secs_f64());
    report["cleanup_complete"] = json!(cleanup.is_ok());
    report["elapsed_seconds"] = json!(started.elapsed().as_secs_f64());
    if let Err(error) = &cleanup {
        report["status"] = json!("failed");
        report["cleanup_error"] = json!(format!("{error:#}"));
    }
    write_json(&output.join("worker.json"), &report)?;
    result?;
    cleanup?;
    Ok(())
}

struct Supervisor {
    owner: Owner,
    focus_file: PathBuf,
    original_focus: [Option<String>; 2],
    focus_changed: bool,
}
impl Supervisor {
    fn new() -> Result<Self> {
        let focus_file = PathBuf::from(std::env::var_os("HOME").context("HOME missing")?)
            .join(".config/kwinrulesrc");
        let text = fs::read_to_string(&focus_file)?;
        let mut section = "";
        let mut entries = std::collections::BTreeMap::new();
        for line in text.lines() {
            if line.starts_with('[') {
                section = line;
            }
            if let Some((key, value)) = line.split_once('=') {
                entries.insert((section.to_string(), key.to_string()), value.to_string());
            }
        }
        let key = |section: &str, name: &str| {
            entries.get(&(section.to_owned(), name.to_owned())).cloned()
        };
        let group = "[hype-harness-startup-focus]";
        ensure!(
            key(group, "wmclass").as_deref() == Some("hype-harness")
                && key(group, "fsplevel").as_deref() == Some("4")
                && key(group, "fsplevelrule").as_deref() == Some("2")
                && key("[General]", "rules")
                    .is_some_and(|s| s.split(',').any(|v| v == "hype-harness-startup-focus")),
            "prepared harness-specific KWin focus rule is missing"
        );
        Ok(Self {
            owner: Owner::new()?,
            focus_file,
            original_focus: [key(group, "acceptfocus"), key(group, "acceptfocusrule")],
            focus_changed: false,
        })
    }
    fn command(&mut self, command: &mut Command) -> Result<()> {
        let process = self
            .owner
            .spawn(command.stdout(Stdio::null()).stderr(Stdio::null()))?;
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = self.owner.try_wait(&process)? {
                ensure!(status.success(), "focus helper failed: {status}");
                return Ok(());
            }
            ensure!(Instant::now() < deadline, "focus helper deadline exceeded");
            thread::sleep(Duration::from_millis(10));
        }
    }
    fn focus(&mut self, enable: bool) -> Result<()> {
        if !enable && !self.focus_changed {
            return Ok(());
        }
        self.focus_changed = true;
        for (index, key) in ["acceptfocus", "acceptfocusrule"].into_iter().enumerate() {
            let value = if enable {
                Some(if index == 0 { "false" } else { "2" }.to_owned())
            } else {
                self.original_focus[index].clone()
            };
            let mut command = Command::new("kwriteconfig6");
            command.arg("--file").arg(&self.focus_file).args([
                "--group",
                "hype-harness-startup-focus",
                "--key",
                key,
            ]);
            if let Some(value) = value {
                command.arg(value);
            } else {
                command.arg("--delete");
            }
            self.command(&mut command)?;
        }
        self.command(Command::new("qdbus6").args(["org.kde.KWin", "/KWin", "reconfigure"]))?;
        self.focus_changed = enable;
        Ok(())
    }
    fn close(&mut self) -> Result<()> {
        let focus = self.focus(false);
        let children = self.owner.close();
        focus?;
        children
    }
}
impl Drop for Supervisor {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

/// Owns a bounded inspection worker and restores the existing focus rule even
/// when that worker crashes or hangs. Both native and reference controllers use
/// the prepared installation's same advisory lock; its service is not stopped.
pub fn inspect(config_path: &Path, output: &Path, fail_stopped_worker: bool) -> Result<()> {
    supervise(config_path, output, fail_stopped_worker, false)
}
pub fn replay(config_path: &Path, output: &Path) -> Result<()> {
    supervise(config_path, output, false, true)
}
fn supervise(
    config_path: &Path,
    output: &Path,
    fail_stopped_worker: bool,
    replay: bool,
) -> Result<()> {
    let cancel = Cancellation::new()?;
    let config_path = config_path.canonicalize()?;
    let lock = File::open(
        config_path
            .parent()
            .context("configuration parent")?
            .join("lock"),
    )?;
    // SAFETY: flock operates only on this open installation lock descriptor.
    ensure!(
        unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0,
        "prepared original-game installation is already in use"
    );
    let config = Config::load(&config_path)?;
    // Refuse a leftover instance, including one whose former owner was killed.
    for entry in fs::read_dir("/proc")? {
        let entry = entry?;
        if !entry.file_name().as_bytes().iter().all(u8::is_ascii_digit) {
            continue;
        }
        let cmd = fs::read(entry.path().join("cmdline")).unwrap_or_default();
        ensure!(
            cmd.split(|b| *b == 0).next() != Some(config.exe.as_os_str().as_bytes()),
            "private game is already running"
        );
    }
    fs::create_dir(output).context("choose a new evidence directory with an existing parent")?;
    let output = output.canonicalize()?;
    let started = Instant::now();
    let mut supervisor = Supervisor::new()?;
    let result = (|| -> Result<()> {
        supervisor.focus(true)?;
        let log = File::create_new(output.join("gamescope.log"))?;
        let mut command = Command::new("gamescope");
        command
            .args([
                "--backend",
                "sdl",
                "-w",
                "1040",
                "-h",
                "800",
                "-W",
                "1040",
                "-H",
                "800",
                "-r",
                "60",
                "-o",
                "60",
                "--",
            ])
            .arg(std::env::current_exe()?)
            .arg("original-worker")
            .arg("--config")
            .arg(&config_path)
            .arg("--output")
            .arg(&output)
            .arg("--host-display")
            .arg(std::env::var("DISPLAY").unwrap_or_default())
            .env("SDL_VIDEODRIVER", "wayland")
            .env("SDL_VIDEO_WAYLAND_WMCLASS", "hype-harness")
            .env("SDL_APP_ID", "hype-harness")
            .env("SDL_WINDOW_NO_ACTIVATION_WHEN_SHOWN", "1")
            .env_remove("XDG_ACTIVATION_TOKEN")
            .env_remove("DESKTOP_STARTUP_ID")
            .stdout(Stdio::from(log.try_clone()?))
            .stderr(Stdio::from(log));
        if fail_stopped_worker {
            command.arg("--fail-stopped-worker");
        }
        if replay {
            command.arg("--replay");
        }
        let gamescope = supervisor.owner.spawn(&mut command)?;
        loop {
            cancel.check()?;
            supervisor.owner.processes()?;
            if output.join("menu-ready.json").exists() {
                supervisor.focus(false)?;
            }
            if let Some(status) = supervisor.owner.try_wait(&gamescope)? {
                ensure!(
                    status.success(),
                    "Gamescope/worker failed: {status}; see gamescope.log"
                );
                break;
            }
            // Gamescope can wait for surviving descendants after its primary
            // worker dies. Observe the worker itself so a stopped game cannot
            // hide its owner's death until the whole-scenario deadline.
            let worker_path = output.join("worker-started.json");
            if worker_path.exists() && !output.join("worker.json").exists() {
                let identity: Identity = serde_json::from_slice(&fs::read(worker_path)?)?;
                let owned = supervisor.owner.processes()?;
                ensure!(
                    owned
                        .iter()
                        .any(|p| p.identity() == identity && p.check().is_ok()),
                    "native worker exited before publishing its result"
                );
            }
            ensure!(
                started.elapsed() < Duration::from_secs(180),
                "native inspection worker deadline exceeded"
            );
            cancel.wait(Duration::from_millis(25))?;
        }
        let report: serde_json::Value =
            serde_json::from_slice(&fs::read(output.join("worker.json"))?)?;
        ensure!(
            report["status"]
                == if replay {
                    "replay_complete"
                } else {
                    "inspection_complete"
                }
                && report["cleanup_complete"] == true,
            "native worker did not finish and clean up"
        );
        Ok(())
    })();
    let cleanup = supervisor.close();
    write_json(
        &output.join("summary.json"),
        &json!({"schema":1,"status":if result.is_ok() && cleanup.is_ok() {"passed"} else {"failed"},
        "scope":if replay {"native original-game physical-input replay"} else {"native original-game inspection, not frame replay or parity"}, "elapsed_seconds":started.elapsed().as_secs_f64(),
        "cleanup_complete":cleanup.is_ok(),"error":result.as_ref().err().map(|e|format!("{e:#}")),"cleanup_error":cleanup.as_ref().err().map(|e|format!("{e:#}")),"worker_report":"worker.json"}),
    )?;
    result?;
    cleanup?;
    Ok(())
}

#[cfg(test)]
mod probe_tests {
    use super::*;
    use std::{os::unix::fs::PermissionsExt, process::Command};

    const ELF_BASE: u32 = 0x0040_0000;
    const ENTRY: u32 = 0x0040_1000;
    const TIME_STUB: u32 = 0x0040_1100;
    const START: u32 = 0x0070_3000;
    const DONE: u32 = 0x0070_3001;
    const UPDATE_CALLS: u32 = 0x0070_3004;
    const ACTOR_ENTRY: u32 = 0x0070_4000;
    const ACTOR_NODE: u32 = 0x0070_4100;
    const ACTOR_PERSO: u32 = 0x0070_4200;
    const ACTOR_DATA3D: u32 = 0x0070_4300;
    const CURRENT_ACTION: u32 = 0x0070_4400;
    const SYNTHETIC_LEFT_ACTION_COUNTER: u32 = 0x0070_4500;
    const IMAGE_END: u32 = 0x0088_0000;

    fn at(image: &mut [u8], address: u32, bytes: &[u8]) {
        let offset = (address - ELF_BASE) as usize;
        image[offset..offset + bytes.len()].copy_from_slice(bytes);
    }
    fn put_u32(image: &mut [u8], address: u32, value: u32) {
        at(image, address, &value.to_le_bytes());
    }
    fn synthetic_elf() -> Vec<u8> {
        let mut image = vec![0; (IMAGE_END - ELF_BASE) as usize];
        image[..7].copy_from_slice(b"\x7fELF\x01\x01\x01");
        image[16..18].copy_from_slice(&2u16.to_le_bytes());
        image[18..20].copy_from_slice(&3u16.to_le_bytes());
        image[20..24].copy_from_slice(&1u32.to_le_bytes());
        image[24..28].copy_from_slice(&ENTRY.to_le_bytes());
        image[28..32].copy_from_slice(&52u32.to_le_bytes());
        image[40..42].copy_from_slice(&52u16.to_le_bytes());
        image[42..44].copy_from_slice(&32u16.to_le_bytes());
        image[44..46].copy_from_slice(&1u16.to_le_bytes());
        image[52..56].copy_from_slice(&1u32.to_le_bytes());
        image[60..64].copy_from_slice(&ELF_BASE.to_le_bytes());
        image[64..68].copy_from_slice(&ELF_BASE.to_le_bytes());
        let size = image.len() as u32;
        image[68..72].copy_from_slice(&size.to_le_bytes());
        image[72..76].copy_from_slice(&size.to_le_bytes());
        image[76..80].copy_from_slice(&7u32.to_le_bytes());
        image[80..84].copy_from_slice(&4096u32.to_le_bytes());

        let mut entry = vec![0x80, 0x3d];
        imm(&mut entry, START);
        entry.extend_from_slice(&[0, 0x75, 9, 0xb8]);
        imm(&mut entry, 20); // getpid: bounded polling without an unsupported i386 syscall
        entry.extend_from_slice(&[0xcd, 0x80, 0xeb, 0xee, 0xb9]);
        imm(&mut entry, RECORD_COUNT as u32);
        let loop_start = entry.len();
        entry.extend_from_slice(&[0xff, 0x15]);
        imm(&mut entry, hype_runtime::UPDATE_CALLBACK_SLOT);
        entry.extend_from_slice(&[0x49, 0x75]);
        let back = entry.len();
        entry.push(0);
        let displacement: i8 = (loop_start as isize - entry.len() as isize)
            .try_into()
            .unwrap();
        entry[back] = displacement as u8;
        entry.extend_from_slice(&[0xc6, 0x05]);
        imm(&mut entry, DONE);
        entry.extend_from_slice(&[1, 0xb8]);
        imm(&mut entry, 20);
        entry.extend_from_slice(&[0xcd, 0x80, 0xeb, 0xf7]);
        at(&mut image, ENTRY, &entry);

        let mut update = vec![0xff, 0x05];
        imm(&mut update, UPDATE_CALLS);
        update.extend_from_slice(&[0xff, 0x05]);
        imm(&mut update, hype_runtime::UPDATE_COUNTER);
        update.push(0xc3);
        at(&mut image, hype_runtime::UPDATE_CALLBACK, &update);
        let mut timer = vec![0xff, 0x05];
        imm(&mut timer, hype_runtime::CLOCK_MS);
        timer.push(0xa1);
        imm(&mut timer, hype_runtime::CLOCK_MS);
        timer.extend_from_slice(&[0xb9]);
        imm(&mut timer, 0xdead_beef);
        timer.push(0xba);
        imm(&mut timer, 0xcafe_babe);
        timer.push(0xc3);
        at(&mut image, TIME_STUB, &timer);

        put_u32(
            &mut image,
            hype_runtime::UPDATE_CALLBACK_SLOT,
            hype_runtime::UPDATE_CALLBACK,
        );
        put_u32(&mut image, 0x005c_0810, TIME_STUB);
        at(&mut image, hype_runtime::STATUS, &[9]);
        put_u32(&mut image, hype_runtime::UPDATE_COUNTER, 100);
        put_u32(&mut image, hype_runtime::CLOCK_MS, 1000);
        put_u32(&mut image, hype_runtime::RAW_DELTA_MS, 20);
        put_u32(&mut image, hype_runtime::CLAMPED_DELTA_MS, 20);
        put_u32(&mut image, hype_runtime::MAIN_ACTOR_ROOT, ACTOR_ENTRY);
        put_u32(&mut image, ACTOR_ENTRY + 4, ACTOR_NODE);
        put_u32(&mut image, ACTOR_NODE + 4, ACTOR_PERSO);
        put_u32(&mut image, ACTOR_PERSO, ACTOR_DATA3D);
        put_u32(&mut image, ACTOR_DATA3D + 4, CURRENT_ACTION);
        put_u32(&mut image, SYNTHETIC_LEFT_ACTION_COUNTER, 0);
        image
    }

    fn word(bytes: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
    }

    fn run_synthetic() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let executable = temp.path().join("wrapper-fixture");
        fs::write(&executable, synthetic_elf())?;
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700))?;
        let mut owner = Owner::new()?;
        let process = owner.spawn(&mut Command::new(executable))?;
        let deadline = Instant::now() + Duration::from_secs(2);
        while process.read_running(ELF_BASE as usize, 4).is_err() {
            if process.check().is_err() {
                anyhow::bail!(
                    "ELF32 fixture exited before mapping: {:?}",
                    owner.try_wait(&process).ok().flatten()
                );
            }
            ensure!(Instant::now() < deadline, "ELF32 fixture did not map");
            thread::sleep(Duration::from_millis(1));
        }
        let mut scratch = Scratch::allocate(&process, 4096)?;
        scratch.write(
            0,
            &observation_probe_image(scratch.address() as u32, SYNTHETIC_LEFT_ACTION_COUNTER)?,
        )?;
        scratch.install_callback(
            hype_runtime::UPDATE_CALLBACK_SLOT as usize,
            hype_runtime::UPDATE_CALLBACK,
            OBSERVATION_CODE,
        )?;
        process.write_instrumentation(START as usize, &[1])?;
        loop {
            if process.read_running(DONE as usize, 1)? == [1] {
                break;
            }
            ensure!(
                Instant::now() < deadline,
                "ELF32 wrapper fixture deadline expired"
            );
            thread::sleep(Duration::from_millis(1));
        }
        let state = scratch.read(STATE, RECORDS + RECORD_COUNT * RECORD_SIZE - STATE)?;
        assert_eq!(word(&state, 8), RECORD_COUNT as u32);
        assert_eq!(word(&state, 12), RECORD_COUNT as u32);
        assert_eq!(word(&state, 16), 0);
        assert_eq!(
            word(&process.read_running(UPDATE_CALLS as usize, 4)?, 0),
            RECORD_COUNT as u32
        );
        for index in 0..RECORD_COUNT {
            let record = RECORDS - STATE + index * RECORD_SIZE;
            assert_eq!(word(&state, record), index as u32 + 1);
            assert_eq!(word(&state, record + 36), CURRENT_ACTION);
        }
        scratch.close()?;
        owner.close()
    }

    #[test]
    fn generated_probe_is_observation_only_and_bounded() -> Result<()> {
        let base = 0x1234_0000;
        let image = observation_probe_image(base, SYNTHETIC_LEFT_ACTION_COUNTER)?;
        assert_eq!(image.len(), 4096);
        assert_eq!(word(&image, STATE + 8), RECORD_COUNT as u32);
        assert_eq!(image[OBSERVATION_CODE], 0xb8);
        assert_eq!(
            &image[OBSERVATION_CODE + 1..OBSERVATION_CODE + 5],
            &hype_runtime::UPDATE_CALLBACK.to_le_bytes()
        );
        assert!(
            !image
                .windows(4)
                .any(|bytes| bytes == hype_runtime::KEYBOARD_CALLBACK.to_le_bytes().as_slice()),
            "observation probe must not call or patch the keyboard callback"
        );
        assert!(image[OBSERVATION_CODE..STATE].iter().any(|byte| *byte != 0));
        assert!(image[RECORDS..].iter().all(|byte| *byte == 0));
        let members: Vec<_> = (0..389_u32)
            .map(|id| format!("live:{:08x}", 0x0200_0000 + id * 4))
            .collect();
        let masks = [0, 0, 1, 0, 1, 0, 0, 3, 3, 3, 3, 3, 0, 0, 0, 0, 0, 0];
        let observations: Vec<_> = masks
            .iter()
            .enumerate()
            .map(|(index, &mask)| {
                let action = match mask {
                    0 => IDLE_ACTION,
                    1 => LEFT_ACTION,
                    3 => SHIFT_LEFT_ACTION,
                    _ => unreachable!(),
                };
                (
                    index as u32 + 1,
                    mask,
                    0x0200_0000 + action * 4,
                    if mask & 1 == 0 { u32::MAX } else { 1 },
                )
            })
            .collect();
        assert_eq!(
            qualify_boundary_consumption(&members, &observations)?,
            masks
                .iter()
                .map(|mask| match mask {
                    0 => IDLE_ACTION,
                    1 => LEFT_ACTION,
                    3 => SHIFT_LEFT_ACTION,
                    _ => unreachable!(),
                })
                .collect::<Vec<_>>()
        );
        let mut wrong_shift = observations.clone();
        wrong_shift[7].2 = 0x0200_0000 + LEFT_ACTION * 4;
        assert!(
            format!(
                "{:#}",
                qualify_boundary_consumption(&members, &wrong_shift).unwrap_err()
            )
            .contains("physical input was not consumed")
        );
        let mut wrong_counter = observations;
        wrong_counter[2].3 = u32::MAX;
        assert!(
            format!(
                "{:#}",
                qualify_boundary_consumption(&members, &wrong_counter).unwrap_err()
            )
            .contains("Left consumer counter disagrees")
        );
        Ok(())
    }

    #[test]
    fn relative_branch_encoder_accepts_forward_and_backward_targets() -> Result<()> {
        let mut bytes = vec![0xeb, 0, 0, 0, 0];
        patch_rel8(&mut bytes, 1, 5)?;
        assert_eq!(bytes[1] as i8, 3);
        patch_rel8(&mut bytes, 4, 1)?;
        assert_eq!(bytes[4] as i8, -4);
        assert!(patch_rel8(&mut bytes, 1, 200).is_err());
        Ok(())
    }

    #[test]
    fn elf32_observation_wrapper_forwards_gameplay_callback_exactly_once() -> Result<()> {
        run_synthetic()
    }
}
