# Native comparison checkpoints

This is a checkpoint ledger for the bounded native comparison work. It records
what has evidence and what remains an acceptance gate; it is not a completion
claim.

## Launch and ownership

- The implementation in commit `bbcc47a` supports launching the supplied
  English Hype fixture and selecting the native runtime inspection. The
  corrected selection passed **18 Rust tests and 14 Python checks in 107.862
  seconds**. The retained report is
  `TestResults/native-runtime-inspection-20260921-fixed.log.json`.
- The earlier attempt selected the same work but timed out at **287.322
  seconds** while testing worker death. Its report remains at
  `TestResults/native-runtime-inspection-20260921.log.json`; the failure is
  retained evidence, not a superseded pass.
- The fixed run records the original inspection at **20.130 seconds** and the
  deliberate worker-death cleanup at **11.212 seconds**. It reports no leaked or
  remaining processes, unchanged saves, and restoration of the existing focus
  rule.
- The original screenshot and stopped-byte capture are independent observations
  of the fixture. They do not establish completed-update publication, live
  identity, or gameplay parity.

## Provenance

- The fixed report identifies source revision `8060f0c` and
  `source_unchanged: true`. Its source digest covers all implementation files,
  including untracked files and the ignored local helper:
  `2ff7ad66e68c2569760bbc679d8616df16f7b24a9d2cf3b3b58ca68f7f3c8aef`.
- The ignored local `tools/lzo1x/lzo1x` helper has SHA-256
  `df3588fe092ccad39271dc312671f67af9d9638379f3f9e5c5b1aa18593c7f8f`.
- A fresh git archive of `bbcc47a` plus the exact helper recomputed the source
  digest. This is a provenance attestation, not a base hash by itself; these
  values are not new source assets.
- Revision `8060f0c` was the expected test revision before the later commit was
  recorded. No rerun is required solely because the commit was made afterward.
- Commit `89aec5a` added experimental remote-allocation instrumentation and
  commit `4f76657` reverted it. That scratch capability is not accepted
  evidence.

## Reader checkpoint

- Commit `5575fa3` projects the selected actor definition through the shared
  source views. The reader agent reports **8.40 seconds** for the selected
  action-47 projection, direct and reopened-Rete routes, and validation of both
  levels. No retained runner report for that timing was found, so this entry is
  agent-reported rather than a locally archived timing result.
- The reader APIs are useful for the selected projection, but live identity is
  still pending. The projection does not prove completed updates or that a
  live actor is the same source/model identity after replacement.

## Acceptance order after launch

The next gates remain ordered: native input with a completed-update boundary;
direct and reopened-Rete routes against one live actor; duplicate, shared,
replaced, and stale identity cases; the same physical-input tape in both
engines; then broader BDD scenarios and coverage accounting. Passing the launch,
cleanup, or reader checkpoints does not pass these later gates.

The reader checkpoint is complete only for its selected static/projection
evidence. No checkpoint here claims a completed live comparison or a full
gameplay port.
