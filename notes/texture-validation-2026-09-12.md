# Texture validation — 2026-09-12

Dated timing and corpus results retained from the texture implementation report. The stable model and codec contract is in [the texture guide](../docs/openspace/texture.md).

Measured on this local development machine on 2026-09-12: five focused tests passed in under 0.01 seconds; the full source-hash/golden-field/GF/CNT corpus check, including whole-archive comparison with XOR obfuscation removed, passed in 22.71 seconds (23.80 seconds including Cargo build, 416,836 KiB peak child RSS). Commands are `cargo test --offline --test texture` and `cargo test --offline --test texture -- --ignored --nocapture`, run with an overall 540-second deadline during this work.
