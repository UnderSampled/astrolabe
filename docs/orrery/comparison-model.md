# Comparison evidence model

Orrery is optional tooling alongside Astrolabe. It can expose structured observations from a running Hype process and drive ordinary physical input through its owned game session. It is not an OpenSpace engine implementation, and the converter's OpenSpace, Rete, and Godot adapters do not depend on it.

Keep these claims distinct:

- **Data comparison** checks selected file or reopened-Rete values against corresponding values observed in the running game.
- **Single-backend behavior** checks a declared input sequence and outcome on one engine. Orrery can be the OpenSpace backend; Godot can run the same scenario when supported.
- **Cross-backend parity** compares observed outcomes across selected backends, with timing and image evidence declared by the scenario. Report each backend’s pass/fail result independently of the diagnostic parity result.
- **Rebuilt-package acceptance** requires installing generated files in a test copy and observing the original game load and use those files.

The [[docs/orrery/controller|Orrery controller]] supplies the live-game operations used by these checks. A successful controller call or selected field comparison supports only the specific capability it exercised. Stopped-memory captures do not establish completed-update timing. Live controls, typed observations, scenario assertions, cross-engine parity, and rebuilt-package acceptance therefore have separate evidence records and completion states.

The dated [[notes/orrery-live-session-2026-09-23|live session record]] captures tested operations, while the evolving [[notes/realtime-comparison-plan|scenario plan]] tracks suite integration.
