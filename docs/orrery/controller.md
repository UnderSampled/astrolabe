# Orrery native original-game MCP controller

Orrery is Astrolabe's optional native Rust controller for the private original
Hype installation. The Cargo package, library, CLI binary, MCP server identity,
and Gamescope application identity are all named `orrery`. The converter does
not depend on Orrery. The [[docs/orrery/comparison-model|comparison evidence model]] defines which claims its observations can support.

The Python `hype` MCP bridge remains available for optional diagnosis. Orrery
implements its control tools and read-only typed-inspection tools.
The controller exposes game launch, save loading, physical input, selected actor
inspection, screenshots, and cleanup. A dated operation record is linked below.
`inspect_main_actor` pauses the owned game and invokes Astrolabe's shared
OpenSpace reader; Orrery does not maintain a second record decoder. Its result
is a `StoppedBytes` capture with executable/save/process/capture provenance and
capture-local locators. It is not a completed-update observation and must not be
used as frame-consumption evidence. With no arguments the tool preserves its
original action-only response. Supplying `designer_values` (zero to 64 unique
indices) opts into the shared typed AI view: normal/reflex behavior selections
plus only those current designer-memory values. An empty list requests behavior
selections without designer values.

The service binds only `127.0.0.1`, requires one exact bearer token from
an owner-owned mode-0600 file, checks `Host` and optional `Origin`, limits HTTP
bodies to 128 KiB, and serializes controller operations. It owns one private
Gamescope and Wine process tree. Every live-memory operation rechecks the pinned
PID/start-time identity and executable/private-prefix guards. Interactive input
uses XTest on Gamescope's private Xwayland display; Orrery does not write Hype's
keyboard buffer or patch its keyboard callback.

The MCP transport uses `rmcp` 3.4.0, the official Rust SDK maintained by the
Model Context Protocol project. Its Streamable HTTP service owns protocol
negotiation, the initialize/initialized lifecycle, session IDs, notification
responses, and content negotiation; Orrery layers its local security policy and
native controller behind that service.

## Build and run

The prepared installation remains at the sibling harness path;
its game data and token stay untracked:

```sh
cargo build -p orrery --release --locked --offline
install -m 600 /dev/null /home/stephen/Projects/hype-harness/local/orrery-mcp-token
openssl rand -hex 32 > /home/stephen/Projects/hype-harness/local/orrery-mcp-token
target/release/orrery mcp \
  --config /home/stephen/Projects/hype-harness/local/config.json \
  --token-file /home/stephen/Projects/hype-harness/local/orrery-mcp-token \
  --runs /home/stephen/Projects/hype-harness/local/orrery-runs \
  --port 18766
```

The endpoint is `http://127.0.0.1:18766/mcp`. The token file must belong to the
current user and have no group/other permission bits. The server starts without
launching the game; `start_game` creates the private window and `stop_game`
removes the full owned process tree. Dropping or terminating the server also
cleans up its game session.

Restart the MCP service after rebuilding Orrery. An older service can keep
serving the previous binary and attempt to launch its now-deleted executable.
For the supplied English save, allow the title menu to settle, select
**Load game**, open the highlighted **Gogoud's Manoir I** slot, then verify phase 9
and level `manoir` before inspecting gameplay. Send and verify each menu key
separately when diagnosing a failed load; a successful input-delivery result
alone does not prove that the save loaded. For a saved-state behavior probe,
allow at least ten seconds of resumed game time after phase 9 appears, then
verify the saved inventory and weapon before applying an intervention. One
unverified menu sequence reached `manoir` with `selected_weapon=-1` and no
active entries. A screenshot-guided sequence selected **Load Game**, then
the highlighted **Gogoud's Manoir I** slot; after the settle, typed inspection
confirmed selected Weapon 80 at quantity zero and Hype action 47. Phase 9 and
level name alone must not be treated as proof that this slot loaded.
The verified menu setup uses each key for 55 ms followed by an 800 ms release
gap. A shorter gap can leave the next screenshot before the menu visibly
updates. The menu `press_keys` tool uses X11 chord names `Down` and `Return`;
the separate `physical_key_sequence` tool uses typed names `ArrowDown` and
`Enter`. Check each tool response for an error before sending the next menu
key.

After verification, this is the Codex entry to add manually. Replace the sample
value with the exact contents of the token file. Do not include angle brackets.

```toml
[mcp_servers.orrery]
url = "http://127.0.0.1:18766/mcp"
http_headers = { Authorization = "Bearer <64-hex-character-token>" }
```

The implementation deliberately does not edit `~/.codex/config.toml`.

## Compatibility matrix

| Old Hype MCP tool | Orrery implementation and preserved guard |
| --- | --- |
| `status` | Service plus owned-session status; does not launch the game |
| `start_game` | Idempotent 60–43,200 second session; verified fixture and executable; private Gamescope display |
| `stop_game` | Idempotent bounded cleanup of the owned process tree |
| `press_keys` | 1–256 validated names/chords, 20–10,000 ms hold, 0–2,000 ms gap, 25-second action cap, unconditional key release |
| `physical_key_sequence` | 1–64 typed `{key, hold_ms, gap_ms}` steps, 25-second aggregate cap; direct XTest edges with unconditional release and no game-state reads or process pauses |
| `enter_cheat` | 1–64 lowercase letters/digits, unique live binding, ordinary OS input |
| `run_sequence` | 1–64 actions, complete prevalidation, 120-second worst-case budget, four screenshots, 1 MiB read budget, no request interleaving, stop and release on failure |
| `get_state` | Live phase/current/pending level from the exact owned process |
| `get_mana` | Live gauge pointer/current/maximum/spell read |
| `screenshot` | Owned X11 window only; path plus MCP `image/png` content, constrained to the run tree and 8 MiB |
| `list_bindings` | Live action table, keyboard-layout inversion, action/code/address output |
| `inspect_main_actor` | Bounded selected action definitions, state selections and transform decoded through Astrolabe from one stopped-byte capture; optional unique action IDs (1–64), default `[47]`; optional `designer_values` (0–64 unique indices) adds normal/reflex behavior selections and typed current values; `include_camera` adds the active viewport pose/projection; not a completed-update observation |
| `inspect_combat` | Hype's selected action/designer state, active inventory, selected magic name/index/mana and matched inventory code, and laser target/visibility decoded from one stopped-byte capture; reports the selected Weapon ID and each active item's ID/quantity, with the same completed-update limit |
| `inspect_scene_tail` | Last 1–16 DynamicWorld children with checked links, kinds, Perso object types and transforms; reads each Perso's Brain model link when ready. Ordinals and `live:*` model locators are capture-local evidence, not imported asset identities |
| `inspect_scene_actor` | Resolve a rooted scene actor by level/path and checked object types. Optional `include_camera_records` permits an empty action list for camera actors with no family actions, follows the camera link, and captures four bounded source records plus the active viewport camera during the same process pause. Raw words and `live:*` locators are transient research evidence, not Rete values or completed-update publication. |
| `inspect_scene_camera` | Search live ActualWorld for one camera-bearing Perso with the requested type triple. Reject missing, ambiguous or malformed matches. Capture its Brain, linked records and active viewport in one pause. The returned live path is a capture-local search result, not a proven imported asset identity. |
| `read_memory` | Numeric or hex read; MCP hex limit 64 KiB; readable mappings and 32-bit address validation |
| `write_memory` | 4 KiB maximum, expected-old-value comparison while all threads are stopped, read-back before resume |
| `memory_maps` | `/proc` maps for the pinned owned identity only |
| `memory_probe` | Write, deliberate guard rejection, and restoration under one pause before resume |
| `navigate_level` | Installed-level check, phase-9 check, guarded transition writes, 30-second completion deadline |

`run_sequence` supports `keys`, `cheat`, `wait`, `wait_state`, `state`, `mana`,
`screenshot`, `bindings`, `inspect_main_actor`, `inspect_combat`, `inspect_scene_tail`, `inspect_scene_actor`, `inspect_scene_camera`, `read`, `write`, and `level`. Runtime failure records
the completed prefix, stops later steps, reports the skipped count, marks the MCP
result as an error, and releases injected keys. Completed effects are retained.
A client timeout does not cancel a controller operation; inspect `status` or
state before retrying a mutation.

## Verification

### Operation record

A dated exercise of these controller operations, including observations and cleanup evidence, is recorded in [notes](../../notes/orrery-live-session-2026-09-23.md).
