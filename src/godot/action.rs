//! Compile family action decisions to ordinary methods over native animations.
//! Runtime code never traverses a serialized source-state graph.
use crate::model::{character::Family, io::Content};
use anyhow::Result;
use std::{collections::BTreeSet, fmt::Write};

pub fn compile(family: &Family) -> Result<String> {
    family.validate()?;
    let mut out = String::from(
        "extends \"res://addons/astrolabe/character/character.gd\"\n## Compiled family action transitions; playback uses the native AnimationPlayer.\n\nfunc _transition_target(from: int, target: int) -> int:\n    match from:\n",
    );
    for state in &family.states {
        if state.transitions.is_empty() && state.prohibited.is_empty() && state.transition_lock != 1
        {
            continue;
        }
        writeln!(out, "        {}:\n            match target:", state.id)?;
        // Engine list order wins, including an explicit transition to a target
        // also present in the prohibited list (00435250/0044a970).
        let mut seen = BTreeSet::new();
        for t in &state.transitions {
            if seen.insert(t.target) {
                writeln!(out, "                {}: return {}", t.target, t.via)?;
            }
        }
        for target in &state.prohibited {
            if seen.insert(*target) {
                writeln!(out, "                {target}: return -1")?;
            }
        }
        writeln!(
            out,
            "                _: return {}",
            if state.transition_lock == 1 {
                "-1"
            } else {
                "target"
            }
        )?;
    }
    out.push_str("        _: return target\n\nfunc _state_next(id: int) -> int:\n    match id:\n");
    for state in &family.states {
        if let Some(next) = state.next {
            writeln!(out, "        {}: return {next}", state.id)?;
        }
    }
    out.push_str("        _: return -1\n\nfunc _state_repeats(id: int) -> int:\n    match id:\n");
    for state in &family.states {
        if state.repeats != 0 {
            writeln!(out, "        {}: return {}", state.id, state.repeats)?;
        }
    }
    out.push_str("        _: return 0\n");
    Ok(out)
}
