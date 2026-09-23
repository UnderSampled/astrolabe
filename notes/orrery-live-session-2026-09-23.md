# Orrery live MCP session — 2026-09-23


A Luna agent used the registered Orrery Streamable HTTP MCP service to start the
verified Hype executable, load the supplied save into phase 9 `manoir`, inspect
Hype, deliver `Left` for 500 ms through ordinary OS input, capture screenshots,
and stop the owned game. `inspect_main_actor(designer_values: [])` returned
normal selection `{initial: 0, previous: 91, current: 4}` and reflex selection
`{initial: 0, previous: 2, current: 3}`. Before and after input, the actor's
basis changed from a half-turn to approximately 1.75 radians while translation
stayed stable; a subsequent inspection saw action 47. This proves live typed
behavior selection and physical input affecting the observed actor, not a
frame-indexed action trace. The capture reported `StoppedBytes` and
`completed_update: false`. Selected designer values have not yet been checked
against a live, safely identified field index.

The private evidence is under
`/home/stephen/Projects/hype-harness/local/orrery-runs/run-1790139957751400842/`.
Two later screenshots have SHA-256 values
`af285f2fdab61308d5f03d416287539561040e985dda031d819fdf7d8fc95b35`
and `cf055e595b5f1a6a3e80d082876bf93fdf1cfdacd9e84279e78dd00b7811e67e`.
After `stop_game`, MCP status reported `controller.running: false`; installed
`SaveGame0/` members still matched the supplied archive. This verifies the
named MCP capabilities individually. The scenario registry, single-backend
assertions and suite report still need implementation.

The focused offline checks are:

```sh
cargo test -p orrery mcp::tests --locked --offline
cargo clippy -p orrery --locked --all-targets --offline -- -D warnings
cargo build -p orrery --release --locked --offline
```

The protocol tests assert the exact 16 names, schemas/annotations, full-batch
prevalidation, timing/screenshot/read/write bounds, status without game launch,
bearer comparison, Host/Origin policy, and base64 image encoding. The loopback
test uses rmcp's real Streamable HTTP client and server over TCP. It verifies the
initialize → initialized-notification → tools/list lifecycle, session handling,
all 16 discovered tools, bearer rejection, and Origin rejection.
