# Temporary native probe workflow

Use the [test supervisor](testing.md) for declared checks. Before adding a
temporary probe, inspect the current test and helper code: an existing preflight
or fixture may already cover the proposed work. This guide describes how to use
those facilities; the probe still owns its scenario and assertions.

## Choose the existing owner

| Concern | Existing owner |
| --- | --- |
| Selected inventory, prerequisites, job/test deadlines, evidence and outer descendant cleanup | `tools/test.py`, `tools/suites.py`, `tools/supervisor.py` |
| Original-game installation lock, Wine, private display/audio/input, logs and process cleanup | Orrery `GameSession` |
| Disposable installation and complete save-root staging | Orrery `session::prepare_isolated_config` |
| Bounded Godot import/editor/script execution | `astrolabe::godot::run_godot` |
| Godot gameplay display selection | `astrolabe::godot::run_godot_gameplay` |

When another worker owns a live run, prepare a patch in an isolated checkout or
patch workspace. Keep its Cargo target, generated project, temporary files and
evidence separate. Do not replace the active binary, edit the active harness,
reuse its output directory or restart its session. `--list` builds and discovers
tests without executing them; it reports `not_run`. A build or parser check is
preflight evidence, not a behavior pass.

## Checked original AI operand experiments

An original-game experiment may change a script operand in an isolated Orrery
session to distinguish competing API interpretations. Use `prepare_isolated_config`
and `GameSession::write_memory` (expected-old comparison, stopped threads and
read-back), with an unchanged control. Preserve original files and saves. This is
a diagnostic variation, separate from unchanged-source or rebuilt-package acceptance.

Resolve the checked live actor, shared AI model, behavior and script slot first;
record executable/save hashes, script extent and original bytes. Shared script
storage can affect every instance of that model. Keep node count, category,
indentation, sentinel and allocation/layout intact; prefer one scalar operand.
Do not substitute a file address for a live pointer or change a reference whose
binding is unresolved. The encoder and guarded-write primitives exist, but there
is no general script-injection or script-resolution API.

Establish that the edit precedes the relevant execution. A pause after an
initialization call is insufficient for a one-shot test. Record changed bytes,
read-back, reached state and the observations that distinguish the alternatives.
Restore with the expected edited bytes or stop the owned process on uncertainty.
Stopped-byte observations alone do not prove completed-update consumption or
API call timing. Give the authorized native worker exact edits, deadlines and
capture instructions; keep this within the existing probe workflow.

## Parse Godot harnesses before converting a level

Place syntax preflight before source import, Rete setup and full-scene export.
Create a fresh minimal project containing `project.godot` with
`config_version=5` and the actual harness script. If the harness requires
preloads or global script classes, include its smallest required dependency
closure from maintained support assets. Keep the later native scene/behavior
checks as well.

Follow the early preflight in [the routes test](../../tests/routes.rs). In a
Rust test, this function accepts a new directory under the test's owned temp
root and the harness bytes (for example, `include_bytes!("check_probe.gd")`):

```rust
fn check_probe_syntax(
    engine: &str,
    project: &std::path::Path,
    script: &[u8],
) -> anyhow::Result<()> {
    use anyhow::{Context, ensure};
    use std::{fs, time::Duration};

    fs::create_dir(project)?;
    let project = project.canonicalize()?;
    fs::write(project.join("project.godot"), "config_version=5\n")?;
    fs::write(project.join("check_probe.gd"), script)?;
    let parser_log = project.join("godot-parser.log");
    let parser_log_arg = parser_log.to_str().context("non-UTF-8 log path")?;
    astrolabe::godot::run_godot(
        engine,
        &project,
        &[
            "--headless", "--log-file", parser_log_arg,
            "--check-only", "--script", "res://check_probe.gd",
        ],
        Duration::from_secs(10),
    )?;
    ensure!(fs::metadata(&parser_log)?.len() <= 1024 * 1024,
        "Godot syntax log exceeded 1MiB diagnostic limit");
    let diagnostics = fs::read_to_string(&parser_log)?;
    ensure!(!diagnostics.contains("SCRIPT ERROR:") && !diagnostics.contains("ERROR:"),
        "Godot syntax log reported errors: {diagnostics}");
    Ok(())
}
```

`run_godot` already bounds the process and checks its exit status and captured
stdout/stderr. Its capture does not redirect Godot's own engine log: always
pass `--log-file` into the owned project/evidence root for standalone checks.
Default `user://logs` may be unwritable under the sandbox. Keep log-write errors
fatal, preserve diagnostics, and stop before conversion when preflight fails.
The ten-second parser deadline is separate from the test's outer budget,
which must cover setup, conversion, behavior and cleanup.

For a contact reporter, `CollisionObject3D.shape_find_owner` accepts the
integer from `KinematicCollision3D.get_collider_shape_index`. The similarly
named `get_collider_shape` returns an Object. See the Godot 4.7
[collision API](https://docs.godotengine.org/en/4.7/classes/class_kinematiccollision3d.html)
and [shape-owner API](https://docs.godotengine.org/en/4.7/classes/class_collisionobject3d.html).
Use the lookup in [the opening reporter](../../tests/check_gogoud_opening.gd):

```gdscript
var collider := hit.get_collider() as CollisionObject3D
var shape: CollisionShape3D
if collider != null:
    var owner_id := collider.shape_find_owner(hit.get_collider_shape_index())
    shape = collider.shape_owner_get_owner(owner_id) as CollisionShape3D
```

## Supply Orrery's bridge when calling from a probe

Read the [controller guide](../orrery/controller.md) and
[runtime fixture guide](../../reference/README.md#runtime-validation) before
original-game work. Prefer the existing record/replay or suite entry point.
For a direct library caller:

1. Verify the intended executable, fixture hash and complete save root. The
   supplied English root contains `SaveGame0/`; passing the inner slot directory
   to the isolation helper is an error.
2. Call `session::prepare_isolated_config(base_config, source_saves, new_output)`.
   Use the returned config and keep the run root in that disposable output.
3. Build/select the actual Orrery CLI from the same source and Cargo profile.
   From a test or custom executable, call
   `GameSession::start_session_with_bridge(config, runs, duration, bridge)`
   with the returned isolated config. The bridge must implement Orrery's
   `display-bridge --output` command. `start_with_bridge` uses `Config::load`
   and requires the pinned English archive/save layout; it is not the entry
   point for the manifest-based config returned by `prepare_isolated_config`.
   The convenience `start`, `start_session` and `start_recording_session` methods
   use `std::env::current_exe()` as the bridge; a probe/test executable usually
   does not dispatch that command and can relaunch itself instead.
4. Set explicit session/action/state-wait deadlines and an outer supervisor
   budget with cleanup reserved. The session lifetime guard is checked during
   controller operations; it does not stop arbitrary stalled caller code.
5. Verify the reached scene and scenario state, then close the session and
   report cleanup. Preserve failures and name the limits of the observation.

The [save tests](../../crates/behavior-suite/tests/save_original.rs) demonstrate
isolated config staging, selecting the profile's Orrery bridge and closing
sessions. Reuse these APIs rather than adding another Wine, display, audio,
installation-lock or process-cleanup wrapper. Original runs and native Godot
runs should execute only under the task's current execution authorization;
preparing this preflight does not itself launch either engine.
