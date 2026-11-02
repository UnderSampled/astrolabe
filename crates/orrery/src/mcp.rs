//! Streamable HTTP MCP server for Orrery, built on the official Rust MCP SDK.
//! The externally reachable surface remains deliberately small: loopback TCP,
//! exact bearer token, Host/Origin allowlists, bounded bodies, and one
//! serialized native controller.
use crate::{desktop::Desktop, orrery::NativeController};
use anyhow::{Context, Result, bail, ensure};
use axum::{
    Json, Router,
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
        ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerConfig, Tool,
        ToolAnnotations,
    },
    service::{NotificationContext, RequestContext},
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    },
};
use serde_json::{Map, Value, json};
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, TryLockError,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

const MAX_BODY: usize = 131_072;
#[derive(Clone, Copy)]
struct ToolSpec {
    name: &'static str,
    description: &'static str,
    read_only: bool,
    destructive: bool,
    schema: fn() -> Value,
}
fn object(properties: Value, required: &[&str]) -> Value {
    json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
}
fn empty() -> Value {
    object(json!({}), &[])
}
fn start_schema() -> Value {
    object(
        json!({
            "duration_seconds":{"type":"integer","minimum":60,"maximum":43200,"default":3600},
            "display_backend":{"type":"string","enum":["headless"],"default":"headless"}
        }),
        &[],
    )
}
fn keys_schema() -> Value {
    object(
        json!({
            "keys":{"type":"array","items":{"type":"string"},"minItems":1,"maxItems":256},
            "hold_ms":{"type":"integer","minimum":20,"maximum":10000,"default":80},
            "gap_ms":{"type":"integer","minimum":0,"maximum":2000,"default":150}
        }),
        &["keys"],
    )
}
fn physical_key_sequence_schema() -> Value {
    object(
        json!({
            "steps":{"type":"array","minItems":1,"maxItems":64,"items":{"type":"object","properties":{
                "key":{"type":"string","enum":["ArrowLeft","ArrowRight","ArrowUp","ArrowDown","LeftShift","LeftControl","X","Z","Space","Enter"]},
                "hold_ms":{"type":"integer","minimum":20,"maximum":10000},
                "gap_ms":{"type":"integer","minimum":0,"maximum":2000}
            },"required":["key","hold_ms","gap_ms"],"additionalProperties":false}}
        }),
        &["steps"],
    )
}
fn cheat_schema() -> Value {
    object(
        json!({
            "text":{"type":"string","pattern":"^[a-z0-9]{1,64}$"},
            "hold_ms":{"type":"integer","minimum":20,"maximum":10000,"default":80},
            "gap_ms":{"type":"integer","minimum":0,"maximum":2000,"default":150}
        }),
        &["text"],
    )
}
fn sequence_schema() -> Value {
    object(
        json!({"steps":{"type":"array","minItems":1,"maxItems":64,"items":{"type":"object"}}}),
        &["steps"],
    )
}
fn level_schema() -> Value {
    object(
        json!({"name":{"type":"string","pattern":"^[a-z0-9_]{1,30}$"}}),
        &["name"],
    )
}
fn inspect_main_actor_schema() -> Value {
    object(
        json!({
            "actions": {
                "type": "array",
                "items": {"type": "integer", "minimum": 0, "maximum": 4294967295_u64},
                "minItems": 1,
                "maxItems": 64,
                "uniqueItems": true,
                "default": [47]
            },
            "designer_values": {
                "type": "array",
                "items": {"type": "integer", "minimum": 0, "maximum": 4294967295_u64},
                "minItems": 0,
                "maxItems": 64,
                "uniqueItems": true,
                "description": "When present, also return live normal/reflex behavior selections and these current designer-memory values. An empty array requests behavior selections only."
            },
            "include_camera": {
                "type": "boolean",
                "default": false,
                "description": "Also capture the active viewport camera in the same stopped-byte sample as MainActor."
            }
        }),
        &[],
    )
}
fn inspect_combat_schema() -> Value {
    let mut schema = inspect_main_actor_schema();
    if let Some(properties) = schema.get_mut("properties").and_then(Value::as_object_mut) {
        properties.remove("include_camera");
    }
    schema
}
fn inspect_scene_actor_schema() -> Value {
    object(
        json!({
            "level":{"type":"string","pattern":"^[a-z0-9_]{1,30}$"},
            "child_path":{"type":"array","minItems":1,"maxItems":32,"items":{"type":"integer","minimum":0,"maximum":99999}},
            "object_types":{"type":"array","minItems":3,"maxItems":3,"items":{"type":"integer","minimum":0,"maximum":4294967295_u64}},
            "actions":{"type":"array","minItems":0,"maxItems":64,"uniqueItems":true,"items":{"type":"integer","minimum":0,"maximum":4294967295_u64}},
            "designer_values":{"type":"array","maxItems":64,"uniqueItems":true,"items":{"type":"integer","minimum":0,"maximum":4294967295_u64}},
            "include_camera_records":{"type":"boolean","default":false,"description":"Capture this actor's four linked source camera records and the active viewport camera within the same process pause."}
        }),
        &["level", "child_path", "object_types", "actions"],
    )
}
fn inspect_scene_camera_schema() -> Value {
    object(
        json!({
            "level":{"type":"string","pattern":"^[a-z0-9_]{1,30}$"},
            "object_types":{"type":"array","minItems":3,"maxItems":3,"items":{"type":"integer","minimum":0,"maximum":4294967295_u64}},
            "designer_values":{"type":"array","maxItems":64,"uniqueItems":true,"items":{"type":"integer","minimum":0,"maximum":4294967295_u64}}
        }),
        &["level", "object_types"],
    )
}
fn inspect_scene_tail_schema() -> Value {
    object(
        json!({"limit":{"type":"integer","minimum":1,"maximum":16,"default":8}}),
        &[],
    )
}
fn read_schema() -> Value {
    object(
        json!({
            "address":{"type":"string","pattern":"^(0x[0-9a-fA-F]+|[0-9]+)$"},
            "type":{"type":"string","enum":["hex","u8","u16","u32","i32","f32"],"default":"hex"},
            "size":{"type":"integer","minimum":1,"maximum":65536,"default":16}
        }),
        &["address"],
    )
}
fn write_schema() -> Value {
    object(
        json!({
            "address":{"type":"string","pattern":"^(0x[0-9a-fA-F]+|[0-9]+)$"},
            "type":{"type":"string","enum":["hex","u8","u16","u32","i32","f32"],"default":"hex"},
            "value":{"type":"string","maxLength":8192},"expect":{"type":"string","maxLength":8192}
        }),
        &["address", "value", "expect"],
    )
}

const TOOLS: [ToolSpec; 23] = [
    ToolSpec {
        name: "status",
        description: "Report the Orrery MCP service and exact owned game status without opening a game window.",
        read_only: true,
        destructive: false,
        schema: empty,
    },
    ToolSpec {
        name: "start_game",
        description: "Open the private game in Gamescope's headless backend and wait for its main menu. Gamescope provides the private Xwayland display used for input and capture. Idempotent while running.",
        read_only: false,
        destructive: false,
        schema: start_schema,
    },
    ToolSpec {
        name: "stop_game",
        description: "Stop Orrery's private game and display while leaving this MCP service running.",
        read_only: false,
        destructive: false,
        schema: empty,
    },
    ToolSpec {
        name: "press_keys",
        description: "Send bounded X11 key names or chords through XTest to the owned private display, releasing every key on success or failure.",
        read_only: false,
        destructive: false,
        schema: keys_schema,
    },
    ToolSpec {
        name: "physical_key_sequence",
        description: "Send bounded physical XTest key taps without reading game state or pausing the process; every key is released on success or failure.",
        read_only: false,
        destructive: false,
        schema: physical_key_sequence_schema,
    },
    ToolSpec {
        name: "enter_cheat",
        description: "Type one unique lowercase live engine binding through the ordinary OS keyboard path and report its before/after state.",
        read_only: false,
        destructive: false,
        schema: cheat_schema,
    },
    ToolSpec {
        name: "run_sequence",
        description: "Validate and run an atomic batch of keys, cheat, wait, wait_state, state, mana, screenshot, bindings, read, write, or level actions. Runtime failure stops later steps and releases keys.",
        read_only: false,
        destructive: true,
        schema: sequence_schema,
    },
    ToolSpec {
        name: "get_state",
        description: "Read engine phase, current level, and pending level from the exact owned game.",
        read_only: true,
        destructive: false,
        schema: empty,
    },
    ToolSpec {
        name: "get_mana",
        description: "Read current and maximum mana, selected spell, and live gauge address during gameplay.",
        read_only: true,
        destructive: false,
        schema: empty,
    },
    ToolSpec {
        name: "screenshot",
        description: "Capture only the owned game window and return its PNG as MCP image content plus a local path.",
        read_only: true,
        destructive: false,
        schema: empty,
    },
    ToolSpec {
        name: "list_bindings",
        description: "Read the live engine cheat sequences, action names, and runtime addresses.",
        read_only: true,
        destructive: false,
        schema: empty,
    },
    ToolSpec {
        name: "inspect_main_actor",
        description: "Pause the owned game, decode the main actor through Astrolabe's shared OpenSpace reader, and return a bounded stopped-byte observation with provenance. It is not a completed-update observation.",
        read_only: true,
        destructive: false,
        schema: inspect_main_actor_schema,
    },
    ToolSpec {
        name: "inspect_combat",
        description: "Pause the owned game once and decode Hype's selected AI fields, active inventory, selected magic and laser through the verified OpenSpace reader.",
        read_only: true,
        destructive: false,
        schema: inspect_combat_schema,
    },
    ToolSpec {
        name: "prepare_crossbow_quantity",
        description: "For the verified manor save, resolve active Weapon 80 inside the paused owned game, guard quantity zero and set a volatile test starting value of three; do not alter save files.",
        read_only: false,
        destructive: true,
        schema: empty,
    },
    ToolSpec {
        name: "inspect_scene_tail",
        description: "Pause the owned game once and list the last 1–16 DynamicWorld children with checked kind, Perso object types, transforms and capture-local AI model links.",
        read_only: true,
        destructive: false,
        schema: inspect_scene_tail_schema,
    },
    ToolSpec {
        name: "inspect_scene_actor",
        description: "Pause the owned game and inspect one source-rooted Perso by a checked child path and object types; all addresses are capture-local.",
        read_only: true,
        destructive: false,
        schema: inspect_scene_actor_schema,
    },
    ToolSpec {
        name: "inspect_scene_camera",
        description: "Search the live ActualWorld tree for one checked camera-bearing Perso, then capture its AI, linked records and active viewport in one pause.",
        read_only: true,
        destructive: false,
        schema: inspect_scene_camera_schema,
    },
    ToolSpec {
        name: "find_scene_actor",
        description: "Pause the owned game and inspect the live actor with these object types, wherever runtime reparenting has moved it.",
        read_only: true,
        destructive: false,
        schema: inspect_scene_camera_schema,
    },
    ToolSpec {
        name: "read_memory",
        description: "Read at most 64 KiB from readable mappings of the exact owned 32-bit game.",
        read_only: true,
        destructive: false,
        schema: read_schema,
    },
    ToolSpec {
        name: "write_memory",
        description: "Guarded expected-old-value write of at most 4 KiB to the paused exact owned game, with read-back verification before resume.",
        read_only: false,
        destructive: true,
        schema: write_schema,
    },
    ToolSpec {
        name: "memory_maps",
        description: "List mapped address ranges and permissions for the exact owned game process.",
        read_only: true,
        destructive: false,
        schema: empty,
    },
    ToolSpec {
        name: "memory_probe",
        description: "Test expected-value guarded access in the pending-level buffer and restore it before the game resumes.",
        read_only: false,
        destructive: true,
        schema: empty,
    },
    ToolSpec {
        name: "navigate_level",
        description: "Request an installed level through a guarded logged memory transition. Requires gameplay phase 9 and retains story state.",
        read_only: false,
        destructive: true,
        schema: level_schema,
    },
];

fn tool_models() -> Vec<Tool> {
    TOOLS
        .iter()
        .map(|spec| {
            let schema = (spec.schema)()
                .as_object()
                .expect("tool schemas are objects")
                .clone();
            Tool::new(spec.name, spec.description, Arc::new(schema)).annotate(
                ToolAnnotations::new()
                    .read_only(spec.read_only)
                    .destructive(spec.destructive)
                    .idempotent(matches!(
                        spec.name,
                        "status"
                            | "start_game"
                            | "stop_game"
                            | "get_state"
                            | "get_mana"
                            | "list_bindings"
                            | "inspect_main_actor"
                            | "inspect_combat"
                            | "inspect_scene_tail"
                            | "inspect_scene_actor"
                            | "inspect_scene_camera"
                            | "find_scene_actor"
                            | "read_memory"
                            | "memory_maps"
                    ))
                    .open_world(false),
            )
        })
        .collect()
}

struct ToolOutput {
    value: Value,
    screenshots: Vec<PathBuf>,
    error: bool,
}
impl ToolOutput {
    fn ok(value: Value) -> Self {
        Self {
            value,
            screenshots: vec![],
            error: false,
        }
    }
    fn result(self, runs: &Path) -> Result<CallToolResult> {
        let mut content = vec![ContentBlock::text(serde_json::to_string(&self.value)?)];
        for path in self.screenshots {
            let canonical = path.canonicalize()?;
            ensure!(
                canonical.starts_with(runs.canonicalize()?)
                    && canonical.extension().is_some_and(|e| e == "png"),
                "unexpected screenshot path"
            );
            let bytes = fs::read(&canonical)?;
            ensure!(bytes.len() <= 8 * 1024 * 1024, "screenshot exceeds 8 MiB");
            content.push(ContentBlock::image(base64(&bytes), "image/png"));
        }
        let mut result = CallToolResult::success(content);
        result.structured_content = Some(self.value);
        result.is_error = Some(self.error);
        Ok(result)
    }
}

fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let word = (chunk[0] as u32) << 16
            | (chunk.get(1).copied().unwrap_or(0) as u32) << 8
            | chunk.get(2).copied().unwrap_or(0) as u32;
        out.push(TABLE[((word >> 18) & 63) as usize] as char);
        out.push(TABLE[((word >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((word >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(word & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

fn object_args(value: &Value) -> Result<&Map<String, Value>> {
    value
        .as_object()
        .context("tool arguments must be an object")
}
fn integer(map: &Map<String, Value>, name: &str, default: u64, lo: u64, hi: u64) -> Result<u64> {
    let value = map
        .get(name)
        .map_or(default, |v| v.as_u64().unwrap_or(u64::MAX));
    ensure!((lo..=hi).contains(&value), "{name} outside {lo}..{hi}");
    Ok(value)
}
fn physical_key_sequence_args(
    map: &Map<String, Value>,
) -> Result<Vec<(crate::replay::Key, Duration, Duration)>> {
    let values = map
        .get("steps")
        .and_then(Value::as_array)
        .context("steps must be an array")?;
    ensure!(
        (1..=64).contains(&values.len()),
        "key sequence must contain 1..64 steps"
    );
    let mut total_ms = 0_u128;
    let mut steps = Vec::with_capacity(values.len());
    for value in values {
        let fields = object_args(value)?;
        allowed(fields, &["key", "hold_ms", "gap_ms"])?;
        let key = serde_json::from_value::<crate::replay::Key>(
            fields.get("key").context("missing key")?.clone(),
        )
        .context("key must be a supported physical key name")?;
        let hold = integer(fields, "hold_ms", u64::MAX, 20, 10_000)?;
        let gap = integer(fields, "gap_ms", u64::MAX, 0, 2_000)?;
        total_ms += u128::from(hold) + u128::from(gap);
        ensure!(total_ms <= 25_000, "key sequence exceeds 25 seconds");
        steps.push((key, Duration::from_millis(hold), Duration::from_millis(gap)));
    }
    Ok(steps)
}
fn string<'a>(map: &'a Map<String, Value>, name: &str) -> Result<&'a str> {
    map.get(name)
        .and_then(Value::as_str)
        .with_context(|| format!("missing string {name}"))
}
fn address(text: &str) -> Result<usize> {
    let value = if let Some(hex) = text.strip_prefix("0x") {
        u32::from_str_radix(hex, 16)?
    } else {
        text.parse::<u32>()?
    };
    Ok(value as usize)
}
fn allowed(map: &Map<String, Value>, fields: &[&str]) -> Result<()> {
    ensure!(
        map.keys().all(|key| fields.contains(&key.as_str())),
        "unknown argument field"
    );
    Ok(())
}
fn inspect_actions(map: &Map<String, Value>) -> Result<Vec<u32>> {
    inspect_actions_allow_empty(map, false)
}
fn inspect_actions_allow_empty(map: &Map<String, Value>, allow_empty: bool) -> Result<Vec<u32>> {
    let Some(values) = map.get("actions") else {
        return Ok(if allow_empty { vec![] } else { vec![47] });
    };
    let values = values.as_array().context("actions must be an array")?;
    ensure!(
        values.len() <= 64 && (allow_empty || !values.is_empty()),
        "actions must have {}..64 items",
        if allow_empty { 0 } else { 1 }
    );
    let actions = values
        .iter()
        .map(|value| {
            value
                .as_u64()
                .and_then(|value| u32::try_from(value).ok())
                .context("action id must be an unsigned 32-bit integer")
        })
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        actions
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            == actions.len(),
        "duplicate action id"
    );
    Ok(actions)
}

fn scene_selector(map: &Map<String, Value>) -> Result<(String, Vec<u32>, [u32; 3])> {
    let (level, object_types) = scene_camera_selector(map)?;
    let path: Vec<u32> = serde_json::from_value(
        map.get("child_path")
            .context("missing scene child path")?
            .clone(),
    )?;
    ensure!(
        (1..=32).contains(&path.len()) && path.iter().all(|index| *index < 100_000),
        "invalid scene child path"
    );
    Ok((level, path, object_types))
}

fn scene_camera_selector(map: &Map<String, Value>) -> Result<(String, [u32; 3])> {
    let level = string(map, "level")?;
    ensure!(
        (1..=30).contains(&level.len())
            && level
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
        "invalid scene actor level"
    );
    let object_types: [u32; 3] = serde_json::from_value(
        map.get("object_types")
            .context("missing scene object types")?
            .clone(),
    )?;
    Ok((level.to_owned(), object_types))
}

fn inspect_designer_values(map: &Map<String, Value>) -> Result<Option<Vec<u32>>> {
    let Some(values) = map.get("designer_values") else {
        return Ok(None);
    };
    let values = values
        .as_array()
        .context("designer_values must be an array")?;
    ensure!(
        values.len() <= 64,
        "designer_values must have at most 64 items"
    );
    let values = values
        .iter()
        .map(|value| {
            value
                .as_u64()
                .and_then(|value| u32::try_from(value).ok())
                .context("designer value id must be an unsigned 32-bit integer")
        })
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        values
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            == values.len(),
        "duplicate designer value id"
    );
    Ok(Some(values))
}

#[derive(Clone)]
struct Step {
    action: String,
    raw: Value,
    delay: u64,
    label: Option<String>,
    budget: u64,
}
fn validate_steps(raw: &Value) -> Result<Vec<Step>> {
    let rows = raw.as_array().context("steps must be an array")?;
    ensure!(
        (1..=64).contains(&rows.len()),
        "sequence must have 1..64 steps"
    );
    ensure!(
        serde_json::to_vec(raw)?.len() <= 64_000,
        "sequence request exceeds 64000 bytes"
    );
    let mut total = 0_u64;
    let mut screenshots = 0;
    let mut reads = 0_usize;
    let mut result = Vec::new();
    for row in rows {
        let map = object_args(row)?;
        let action = string(map, "action")?.to_owned();
        let delay = integer(map, "delay_ms", 0, 0, 30_000)?;
        let label = map
            .get("label")
            .map(|v| v.as_str().context("label must be a string"))
            .transpose()?
            .map(str::to_owned);
        ensure!(
            label.as_ref().is_none_or(|v| v.len() <= 80),
            "label exceeds 80 bytes"
        );
        let mut budget = 500 + delay;
        match action.as_str() {
            "keys" => {
                allowed(
                    map,
                    &["action", "delay_ms", "label", "keys", "hold_ms", "gap_ms"],
                )?;
                let keys = map
                    .get("keys")
                    .and_then(Value::as_array)
                    .context("keys must be an array")?;
                ensure!(
                    (1..=256).contains(&keys.len()),
                    "keys must have 1..256 items"
                );
                let h = integer(map, "hold_ms", 80, 20, 10_000)?;
                let g = integer(map, "gap_ms", 150, 0, 2_000)?;
                ensure!(
                    keys.iter().all(|k| k
                        .as_str()
                        .is_some_and(|s| Desktop::validate_chord(s).is_ok())),
                    "invalid key chord"
                );
                ensure!(
                    keys.len() as u64 * (h + g) <= 25_000,
                    "key action exceeds 25 seconds"
                );
                budget += keys.len() as u64 * (h + g);
            }
            "cheat" => {
                allowed(
                    map,
                    &["action", "delay_ms", "label", "text", "hold_ms", "gap_ms"],
                )?;
                let t = string(map, "text")?;
                ensure!(
                    (1..=64).contains(&t.len())
                        && t.bytes()
                            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()),
                    "invalid cheat text"
                );
                let h = integer(map, "hold_ms", 80, 20, 10_000)?;
                let g = integer(map, "gap_ms", 150, 0, 2_000)?;
                ensure!(
                    t.len() as u64 * (h + g) <= 25_000,
                    "cheat exceeds 25 seconds"
                );
                budget += t.len() as u64 * (h + g);
            }
            "wait" => {
                allowed(map, &["action", "delay_ms", "label", "duration_ms"])?;
                budget = delay + integer(map, "duration_ms", u64::MAX, 0, 30_000)?;
            }
            "wait_state" => {
                allowed(
                    map,
                    &[
                        "action",
                        "delay_ms",
                        "label",
                        "phase",
                        "level",
                        "timeout_ms",
                    ],
                )?;
                ensure!(
                    map.contains_key("phase") || map.contains_key("level"),
                    "wait_state requires phase and/or level"
                );
                if let Some(v) = map.get("phase") {
                    ensure!(v.as_u64().is_some_and(|n| n <= 255), "invalid phase");
                }
                if let Some(v) = map.get("level") {
                    let s = v.as_str().context("level must be string")?;
                    ensure!((1..=30).contains(&s.len()), "invalid level");
                }
                budget += integer(map, "timeout_ms", 15_000, 100, 30_000)?;
            }
            "state" | "mana" | "bindings" => {
                allowed(map, &["action", "delay_ms", "label"])?;
            }
            "inspect_main_actor" | "inspect_combat" => {
                allowed(
                    map,
                    &[
                        "action",
                        "delay_ms",
                        "label",
                        "actions",
                        "designer_values",
                        "include_camera",
                    ],
                )?;
                inspect_actions(map)?;
                inspect_designer_values(map)?;
                ensure!(
                    map.get("include_camera").is_none_or(Value::is_boolean),
                    "include_camera must be Boolean"
                );
                ensure!(
                    action != "inspect_combat" || !map.contains_key("include_camera"),
                    "include_camera is only valid for inspect_main_actor"
                );
                budget += 2_500;
            }
            "inspect_scene_tail" => {
                allowed(map, &["action", "delay_ms", "label", "limit"])?;
                integer(map, "limit", 8, 1, 16)?;
                budget += 2_500;
            }
            "inspect_scene_actor" => {
                allowed(
                    map,
                    &[
                        "action",
                        "delay_ms",
                        "label",
                        "level",
                        "child_path",
                        "object_types",
                        "actions",
                        "designer_values",
                        "include_camera_records",
                    ],
                )?;
                scene_selector(map)?;
                inspect_actions_allow_empty(
                    map,
                    map.get("include_camera_records")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                )?;
                inspect_designer_values(map)?;
                ensure!(
                    map.get("include_camera_records")
                        .is_none_or(Value::is_boolean),
                    "include_camera_records must be Boolean"
                );
                budget += 2_500;
            }
            "find_scene_actor" | "inspect_scene_camera" => {
                allowed(
                    map,
                    &[
                        "action",
                        "delay_ms",
                        "label",
                        "level",
                        "object_types",
                        "designer_values",
                    ],
                )?;
                scene_camera_selector(map)?;
                inspect_designer_values(map)?;
                budget += 5_000;
            }
            "screenshot" => {
                allowed(map, &["action", "delay_ms", "label"])?;
                screenshots += 1;
            }
            "level" => {
                allowed(map, &["action", "delay_ms", "label", "name"])?;
                let n = string(map, "name")?;
                ensure!(
                    (1..=30).contains(&n.len())
                        && n.bytes()
                            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
                    "invalid level name"
                );
                budget = delay + 35_000;
            }
            "read" => {
                allowed(
                    map,
                    &["action", "delay_ms", "label", "address", "type", "size"],
                )?;
                address(string(map, "address")?)?;
                let kind = map.get("type").and_then(Value::as_str).unwrap_or("hex");
                ensure!(
                    ["hex", "u8", "u16", "u32", "i32", "f32"].contains(&kind),
                    "invalid memory type"
                );
                let size = if kind == "hex" {
                    integer(map, "size", 16, 1, 65_536)? as usize
                } else {
                    4
                };
                reads += size;
            }
            "write" => {
                allowed(
                    map,
                    &[
                        "action", "delay_ms", "label", "address", "type", "value", "expect",
                    ],
                )?;
                address(string(map, "address")?)?;
                let kind = map.get("type").and_then(Value::as_str).unwrap_or("hex");
                ensure!(
                    ["hex", "u8", "u16", "u32", "i32", "f32"].contains(&kind),
                    "invalid memory type"
                );
                let value = string(map, "value")?;
                let expect = string(map, "expect")?;
                ensure!(
                    value.len() <= 8192 && expect.len() <= 8192,
                    "memory value too long"
                );
                validate_packed(kind, value, expect)?;
            }
            _ => bail!("unknown sequence action: {action}"),
        }
        total = total
            .checked_add(budget)
            .context("sequence budget overflow")?;
        result.push(Step {
            action,
            raw: row.clone(),
            delay,
            label,
            budget,
        });
    }
    ensure!(
        total <= 120_000,
        "sequence worst-case timing budget exceeds 120 seconds"
    );
    ensure!(screenshots <= 4, "at most four screenshots per sequence");
    ensure!(
        reads <= 1_048_576,
        "sequence memory read budget exceeds 1 MiB"
    );
    Ok(result)
}
fn validate_packed(kind: &str, value: &str, expect: &str) -> Result<()> {
    let parse = |text: &str| -> Result<usize> {
        Ok(match kind {
            "hex" => {
                ensure!(text.len().is_multiple_of(2), "odd hex length");
                for i in (0..text.len()).step_by(2) {
                    u8::from_str_radix(&text[i..i + 2], 16)?;
                }
                text.len() / 2
            }
            "u8" => {
                text.parse::<u8>()?;
                1
            }
            "u16" => {
                text.parse::<u16>()?;
                2
            }
            "u32" => {
                text.parse::<u32>()?;
                4
            }
            "i32" => {
                text.parse::<i32>()?;
                4
            }
            "f32" => {
                text.parse::<f32>()?;
                4
            }
            _ => bail!("invalid memory type"),
        })
    };
    let (a, b) = (parse(value)?, parse(expect)?);
    ensure!(
        a > 0 && a == b && a <= 4096,
        "write values must have equal size in 1..4096"
    );
    Ok(())
}

fn execute_sequence(controller: &mut NativeController, steps: Vec<Step>) -> ToolOutput {
    let started = Instant::now();
    let total = steps.len();
    let mut rows = Vec::new();
    let mut screenshots = Vec::new();
    let mut complete = true;
    for (index, step) in steps.into_iter().enumerate() {
        let mut row =
            json!({"index":index,"action":step.action,"label":step.label,"budget_ms":step.budget});
        thread::sleep(Duration::from_millis(step.delay));
        row["started_ms"] = json!(started.elapsed().as_secs_f64() * 1000.0);
        let value = execute_step(controller, &step);
        match value {
            Ok(value) => {
                if step.action == "screenshot"
                    && let Some(p) = value["path"].as_str()
                {
                    screenshots.push(PathBuf::from(p));
                }
                row["ok"] = json!(true);
                row["result"] = value;
            }
            Err(error) => {
                row["ok"] = json!(false);
                row["error"] = json!(format!("{error:#}"));
                complete = false;
            }
        }
        row["finished_ms"] = json!(started.elapsed().as_secs_f64() * 1000.0);
        rows.push(row);
        if !complete {
            break;
        }
    }
    if let Ok(game) = controller.game() {
        let _ = game.release();
    }
    let count = rows.len();
    ToolOutput {
        value: json!({"completed":complete,"elapsed_ms":started.elapsed().as_secs_f64()*1000.0,"steps":rows,"skipped":total-count}),
        screenshots,
        error: !complete,
    }
}
fn execute_step(controller: &mut NativeController, step: &Step) -> Result<Value> {
    let map = object_args(&step.raw)?;
    let game = controller.game()?;
    match step.action.as_str() {
        "keys" => {
            let keys = map["keys"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_owned())
                .collect::<Vec<_>>();
            game.keys(
                &keys,
                integer(map, "hold_ms", 80, 20, 10_000)?,
                integer(map, "gap_ms", 150, 0, 2_000)?,
            )
        }
        "cheat" => game.cheat(
            string(map, "text")?,
            integer(map, "hold_ms", 80, 20, 10_000)?,
            integer(map, "gap_ms", 150, 0, 2_000)?,
        ),
        "wait" => {
            let ms = integer(map, "duration_ms", 0, 0, 30_000)?;
            thread::sleep(Duration::from_millis(ms));
            Ok(json!({"waited_ms":ms}))
        }
        "wait_state" => {
            let deadline = Instant::now()
                + Duration::from_millis(integer(map, "timeout_ms", 15_000, 100, 30_000)?);
            loop {
                let state = game.state()?;
                let phase_ok = map.get("phase").is_none_or(|v| state["phase"] == *v);
                let level_ok = map.get("level").is_none_or(|v| state["level"] == *v);
                if phase_ok && level_ok {
                    break Ok(state);
                }
                ensure!(
                    Instant::now() < deadline,
                    "state condition not reached; last state: {state}"
                );
                thread::sleep(Duration::from_millis(100));
            }
        }
        "state" => game.state(),
        "mana" => game.mana(),
        "screenshot" => game.screenshot(),
        "bindings" => Ok(json!({"bindings":game.bindings()?})),
        "inspect_main_actor" => {
            let actions = inspect_actions(map)?;
            let designer_values = inspect_designer_values(map)?;
            game.inspect_main_actor_with_camera(
                &actions,
                designer_values.as_deref(),
                map.get("include_camera")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            )
        }
        "inspect_combat" => {
            let actions = inspect_actions(map)?;
            let designer_values = inspect_designer_values(map)?.unwrap_or_default();
            game.inspect_combat(&actions, &designer_values)
        }
        "inspect_scene_tail" => game.inspect_scene_tail(integer(map, "limit", 8, 1, 16)? as usize),
        "inspect_scene_actor" => {
            let (level, path, object_types) = scene_selector(map)?;
            let actions = inspect_actions_allow_empty(
                map,
                map.get("include_camera_records")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            )?;
            let designer_values = inspect_designer_values(map)?.unwrap_or_default();
            game.inspect_scene_actor_with_camera(
                &level,
                &path,
                object_types,
                &actions,
                &designer_values,
                map.get("include_camera_records")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            )
        }
        "find_scene_actor" => {
            let (level, object_types) = scene_camera_selector(map)?;
            let designer_values = inspect_designer_values(map)?.unwrap_or_default();
            game.inspect_scene_actor_by_types(&level, object_types, &[], &designer_values)
        }
        "inspect_scene_camera" => {
            let (level, object_types) = scene_camera_selector(map)?;
            let designer_values = inspect_designer_values(map)?.unwrap_or_default();
            game.inspect_scene_camera(&level, object_types, &designer_values)
        }
        "level" => game.navigate_level(string(map, "name")?),
        "read" => game.read_memory(
            address(string(map, "address")?)?,
            map.get("type").and_then(Value::as_str).unwrap_or("hex"),
            integer(map, "size", 16, 1, 65_536)? as usize,
        ),
        "write" => game.write_memory(
            address(string(map, "address")?)?,
            map.get("type").and_then(Value::as_str).unwrap_or("hex"),
            string(map, "value")?,
            string(map, "expect")?,
        ),
        _ => bail!("unknown action"),
    }
}

fn single_action(controller: &mut NativeController, action: Value) -> Result<ToolOutput> {
    let mut result = execute_sequence(controller, validate_steps(&json!([action]))?);
    if result.error {
        return Ok(result);
    }
    let step = result.value["steps"][0]["result"].take();
    result.value = step;
    Ok(result)
}

fn call_tool(
    controller: &mut NativeController,
    name: &str,
    args: &Value,
    service_pid: u32,
    port: u16,
) -> Result<ToolOutput> {
    let map = object_args(args)?;
    match name {
        "status" => {
            allowed(map, &[])?;
            let status = if controller.running() {
                json!({"running":true,"session":controller.game()?.status()?})
            } else {
                json!({"running":false})
            };
            Ok(ToolOutput::ok(
                json!({"service_pid":service_pid,"transport":"streamable-http","endpoint":format!("http://127.0.0.1:{port}/mcp"),"controller":status}),
            ))
        }
        "prepare_crossbow_quantity" => {
            allowed(map, &[])?;
            Ok(ToolOutput::ok(
                controller.game()?.prepare_crossbow_quantity()?,
            ))
        }
        "start_game" => {
            allowed(map, &["duration_seconds", "display_backend"])?;
            ensure!(
                map.get("display_backend")
                    .and_then(Value::as_str)
                    .unwrap_or("headless")
                    == "headless",
                "only the headless display backend is supported"
            );
            Ok(ToolOutput::ok(controller.start(integer(
                map,
                "duration_seconds",
                3600,
                60,
                43_200,
            )?)?))
        }
        "stop_game" => {
            allowed(map, &[])?;
            Ok(ToolOutput::ok(controller.stop()?))
        }
        "run_sequence" => {
            allowed(map, &["steps"])?;
            Ok(execute_sequence(
                controller,
                validate_steps(map.get("steps").context("missing steps")?)?,
            ))
        }
        "press_keys" => {
            allowed(map, &["keys", "hold_ms", "gap_ms"])?;
            let mut action = args.clone();
            action["action"] = json!("keys");
            single_action(controller, action)
        }
        "physical_key_sequence" => {
            allowed(map, &["steps"])?;
            let steps = physical_key_sequence_args(map)?;
            Ok(ToolOutput::ok(
                controller.game()?.physical_key_sequence(&steps)?,
            ))
        }
        "enter_cheat" => {
            allowed(map, &["text", "hold_ms", "gap_ms"])?;
            let mut action = args.clone();
            action["action"] = json!("cheat");
            single_action(controller, action)
        }
        "get_state" => single_action(controller, json!({"action":"state"})),
        "get_mana" => single_action(controller, json!({"action":"mana"})),
        "screenshot" => single_action(controller, json!({"action":"screenshot"})),
        "list_bindings" => single_action(controller, json!({"action":"bindings"})),
        "inspect_main_actor" => {
            allowed(map, &["actions", "designer_values", "include_camera"])?;
            ensure!(
                map.get("include_camera").is_none_or(Value::is_boolean),
                "include_camera must be Boolean"
            );
            let actions = inspect_actions(map)?;
            let designer_values = inspect_designer_values(map)?;
            Ok(ToolOutput::ok(
                controller.game()?.inspect_main_actor_with_camera(
                    &actions,
                    designer_values.as_deref(),
                    map.get("include_camera")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                )?,
            ))
        }
        "inspect_combat" => {
            allowed(map, &["actions", "designer_values"])?;
            let actions = inspect_actions(map)?;
            let designer_values = inspect_designer_values(map)?.unwrap_or_default();
            Ok(ToolOutput::ok(
                controller
                    .game()?
                    .inspect_combat(&actions, &designer_values)?,
            ))
        }
        "inspect_scene_tail" => {
            allowed(map, &["limit"])?;
            Ok(ToolOutput::ok(controller.game()?.inspect_scene_tail(
                integer(map, "limit", 8, 1, 16)? as usize,
            )?))
        }
        "inspect_scene_actor" => {
            allowed(
                map,
                &[
                    "level",
                    "child_path",
                    "object_types",
                    "actions",
                    "designer_values",
                    "include_camera_records",
                ],
            )?;
            ensure!(
                map.get("include_camera_records")
                    .is_none_or(Value::is_boolean),
                "include_camera_records must be Boolean"
            );
            let (level, path, object_types) = scene_selector(map)?;
            let actions = inspect_actions_allow_empty(
                map,
                map.get("include_camera_records")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            )?;
            let designer_values = inspect_designer_values(map)?.unwrap_or_default();
            Ok(ToolOutput::ok(
                controller.game()?.inspect_scene_actor_with_camera(
                    &level,
                    &path,
                    object_types,
                    &actions,
                    &designer_values,
                    map.get("include_camera_records")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                )?,
            ))
        }
        "find_scene_actor" => {
            allowed(map, &["level", "object_types", "designer_values"])?;
            let (level, object_types) = scene_camera_selector(map)?;
            let designer_values = inspect_designer_values(map)?.unwrap_or_default();
            Ok(ToolOutput::ok(controller.game()?.inspect_scene_actor_by_types(
                &level,
                object_types,
                &[],
                &designer_values,
            )?))
        }
        "inspect_scene_camera" => {
            allowed(map, &["level", "object_types", "designer_values"])?;
            let (level, object_types) = scene_camera_selector(map)?;
            let designer_values = inspect_designer_values(map)?.unwrap_or_default();
            Ok(ToolOutput::ok(controller.game()?.inspect_scene_camera(
                &level,
                object_types,
                &designer_values,
            )?))
        }
        "navigate_level" => {
            allowed(map, &["name"])?;
            single_action(
                controller,
                json!({"action":"level","name":string(map,"name")?}),
            )
        }
        "read_memory" => {
            allowed(map, &["address", "type", "size"])?;
            let mut action = args.clone();
            action["action"] = json!("read");
            single_action(controller, action)
        }
        "write_memory" => {
            allowed(map, &["address", "type", "value", "expect"])?;
            let mut action = args.clone();
            action["action"] = json!("write");
            single_action(controller, action)
        }
        "memory_maps" => {
            allowed(map, &[])?;
            Ok(ToolOutput::ok(
                json!({"maps":controller.game()?.memory_maps()?}),
            ))
        }
        "memory_probe" => {
            allowed(map, &[])?;
            Ok(ToolOutput::ok(controller.game()?.memory_probe()?))
        }
        _ => bail!("unknown tool: {name}"),
    }
}

struct ServerState {
    controller: Mutex<NativeController>,
    runs: PathBuf,
    port: u16,
    initialized_notifications: AtomicUsize,
}
fn lock_controller(state: &ServerState) -> Result<std::sync::MutexGuard<'_, NativeController>> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match state.controller.try_lock() {
            Ok(guard) => return Ok(guard),
            Err(TryLockError::Poisoned(_)) => bail!("controller lock poisoned"),
            Err(TryLockError::WouldBlock) => {
                ensure!(
                    Instant::now() < deadline,
                    "controller busy with another operation; inspect status after it finishes"
                );
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

#[derive(Clone)]
struct OrreryMcp {
    state: Arc<ServerState>,
}

impl ServerHandler for OrreryMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("Orrery", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "Control the private Hype game through Orrery. Sequences are serialized and \
                 validated before mutation; inspect status after any client timeout.",
            )
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        tool_models().into_iter().find(|tool| tool.name == name)
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> std::result::Result<ListToolsResult, McpError> {
        Ok(ListToolsResult::with_all_items(tool_models()))
    }

    async fn on_initialized(&self, _context: NotificationContext<RoleServer>) {
        self.state
            .initialized_notifications
            .fetch_add(1, Ordering::Relaxed);
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> std::result::Result<CallToolResponse, McpError> {
        let state = self.state.clone();
        let name = request.name.into_owned();
        let args = Value::Object(request.arguments.unwrap_or_default());
        let result = tokio::task::spawn_blocking(move || -> Result<CallToolResult> {
            let mut controller = lock_controller(&state)?;
            call_tool(
                &mut controller,
                &name,
                &args,
                std::process::id(),
                state.port,
            )?
            .result(&state.runs)
        })
        .await
        .map_err(|error| {
            McpError::internal_error(
                "Orrery controller task failed",
                Some(json!({"error":error.to_string()})),
            )
        })?;
        match result {
            Ok(result) => Ok(result.into()),
            Err(error) => {
                Ok(CallToolResult::error(vec![ContentBlock::text(format!("{error:#}"))]).into())
            }
        }
    }
}

fn constant_time_equal(a: &[u8], b: &[u8]) -> bool {
    let mut different = a.len() ^ b.len();
    for i in 0..a.len().max(b.len()) {
        different |= usize::from(a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0));
    }
    different == 0
}

#[derive(Clone)]
struct AuthState {
    expected: Arc<[u8]>,
}

async fn require_auth(State(auth): State<AuthState>, request: Request, next: Next) -> Response {
    let values = request.headers().get_all(header::AUTHORIZATION);
    let mut values = values.iter();
    let authorized = values
        .next()
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| constant_time_equal(value.as_bytes(), &auth.expected))
        && values.next().is_none();
    if authorized {
        next.run(request).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, "Bearer")],
            Json(json!({"error":"Local MCP token required"})),
        )
            .into_response()
    }
}

fn router(state: Arc<ServerState>, token: &str) -> Router {
    let port = state.port;
    let handler = OrreryMcp { state };
    let service = StreamableHttpService::new(
        move || Ok(handler.clone()),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default()
            .with_json_response(true)
            .with_allowed_hosts([format!("127.0.0.1:{port}"), format!("localhost:{port}")])
            .with_allowed_origins([
                format!("http://127.0.0.1:{port}"),
                format!("http://localhost:{port}"),
            ])
            .with_max_request_body_bytes(MAX_BODY),
    );
    Router::new()
        .nest_service("/mcp", service)
        .layer(middleware::from_fn_with_state(
            AuthState {
                expected: format!("Bearer {token}").into_bytes().into(),
            },
            require_auth,
        ))
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("install SIGTERM handler");
        tokio::select! {
            result = tokio::signal::ctrl_c() => { let _ = result; }
            _ = terminate.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

pub fn serve(config: PathBuf, runs: PathBuf, token_file: PathBuf, port: u16) -> Result<()> {
    let metadata = fs::metadata(&token_file)?;
    ensure!(
        metadata.is_file()
            && metadata.uid() == unsafe { libc::geteuid() }
            && metadata.permissions().mode() & 0o077 == 0,
        "token file must be owner-owned mode 0600"
    );
    let token = fs::read_to_string(token_file)?.trim().to_owned();
    ensure!(
        token.len() >= 32 && token.is_ascii(),
        "MCP token must contain at least 32 ASCII characters"
    );
    fs::create_dir_all(&runs)?;
    let state = Arc::new(ServerState {
        controller: Mutex::new(NativeController::new(config, runs.clone())),
        runs,
        port,
        initialized_notifications: AtomicUsize::new(0),
    });
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let listener =
                tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
            axum::serve(listener, router(state, &token))
                .with_graceful_shutdown(shutdown_signal())
                .await
                .context("serve Orrery MCP")
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::{
        ServiceExt,
        transport::streamable_http_client::{
            StreamableHttpClientTransport, StreamableHttpClientTransportConfig,
        },
    };

    fn state(port: u16) -> (tempfile::TempDir, Arc<ServerState>) {
        let temp = tempfile::tempdir().unwrap();
        let runs = temp.path().join("runs");
        fs::create_dir(&runs).unwrap();
        let state = Arc::new(ServerState {
            controller: Mutex::new(NativeController::new(
                temp.path().join("unused-config.json"),
                runs.clone(),
            )),
            runs,
            port,
            initialized_notifications: AtomicUsize::new(0),
        });
        (temp, state)
    }
    #[test]
    fn physical_key_sequence_arguments_are_ordered_and_bounded() {
        let value = json!({"steps":[
            {"key":"Z","hold_ms":55,"gap_ms":100},
            {"key":"Space","hold_ms":80,"gap_ms":0}
        ]});
        let steps = physical_key_sequence_args(value.as_object().unwrap()).unwrap();
        assert_eq!(
            steps,
            [
                (
                    crate::replay::Key::Z,
                    Duration::from_millis(55),
                    Duration::from_millis(100)
                ),
                (
                    crate::replay::Key::Space,
                    Duration::from_millis(80),
                    Duration::ZERO
                ),
            ]
        );

        let oversized = json!({"steps":[
            {"key":"Z","hold_ms":10000,"gap_ms":2000},
            {"key":"Space","hold_ms":10000,"gap_ms":2000},
            {"key":"Enter","hold_ms":10000,"gap_ms":2000}
        ]});
        assert!(physical_key_sequence_args(oversized.as_object().unwrap()).is_err());
        let invalid_key = json!({"steps":[{"key":"z","hold_ms":55,"gap_ms":100}]});
        assert!(physical_key_sequence_args(invalid_key.as_object().unwrap()).is_err());
    }

    #[test]
    fn parity_surface_is_exact_and_annotated() {
        let names = TOOLS.iter().map(|t| t.name).collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "status",
                "start_game",
                "stop_game",
                "press_keys",
                "physical_key_sequence",
                "enter_cheat",
                "run_sequence",
                "get_state",
                "get_mana",
                "screenshot",
                "list_bindings",
                "inspect_main_actor",
                "inspect_combat",
                "prepare_crossbow_quantity",
                "inspect_scene_tail",
                "inspect_scene_actor",
                "inspect_scene_camera",
                "find_scene_actor",
                "read_memory",
                "write_memory",
                "memory_maps",
                "memory_probe",
                "navigate_level"
            ]
        );
        assert!(TOOLS.iter().all(|t| !t.description.is_empty()));
        assert!(TOOLS.iter().filter(|t| t.destructive).map(|t| t.name).eq([
            "run_sequence",
            "prepare_crossbow_quantity",
            "write_memory",
            "memory_probe",
            "navigate_level"
        ]));
        let inspect = TOOLS
            .iter()
            .find(|tool| tool.name == "inspect_main_actor")
            .unwrap();
        assert!(inspect.read_only && !inspect.destructive);
        assert_eq!(
            (inspect.schema)()["properties"]["actions"]["default"],
            json!([47])
        );
        let schema = (inspect.schema)();
        assert_eq!(schema["properties"]["designer_values"]["maxItems"], 64);
        assert_eq!(schema["properties"]["designer_values"]["uniqueItems"], true);
        assert_eq!(schema["properties"]["designer_values"]["minItems"], 0);
        let scene = TOOLS
            .iter()
            .find(|tool| tool.name == "inspect_scene_actor")
            .unwrap();
        assert!(scene.read_only && !scene.destructive);
        let camera = TOOLS
            .iter()
            .find(|tool| tool.name == "inspect_scene_camera")
            .unwrap();
        assert!(camera.read_only && !camera.destructive);
    }
    #[test]
    fn entire_sequence_is_rejected_before_execution() {
        assert!(
            validate_steps(
                &json!([{"action":"wait","duration_ms":1},{"action":"keys","keys":["NotAKey"]}])
            )
            .is_err()
        );
        assert!(validate_steps(&json!([{"action":"wait","duration_ms":30000},{"action":"wait_state","phase":9,"timeout_ms":30000},{"action":"level","name":"manoir"},{"action":"level","name":"astrolabe"}])).is_err());
        assert!(validate_steps(&json!([{"action":"screenshot"},{"action":"screenshot"},{"action":"screenshot"},{"action":"screenshot"},{"action":"screenshot"}])).is_err());
        assert!(
            validate_steps(&json!([{"action":"inspect_main_actor","actions":[47,47]}])).is_err()
        );
        assert!(validate_steps(&json!([{"action":"inspect_main_actor","actions":[]}])).is_err());
        assert!(
            validate_steps(&json!([{"action":"inspect_main_actor","actions":[4294967296_u64]}]))
                .is_err()
        );
        assert_eq!(inspect_actions(&Map::new()).unwrap(), vec![47]);
        let valid_scene = json!([{"action":"inspect_scene_actor","level":"manoir","child_path":[0,153],"object_types":[39,62,180],"actions":[0]}]);
        assert!(validate_steps(&valid_scene).is_ok());
        let valid_camera = json!([{"action":"inspect_scene_actor","level":"manoir","child_path":[0,2],"object_types":[2,2,2],"actions":[],"include_camera_records":true}]);
        assert!(validate_steps(&valid_camera).is_ok());
        let default_camera = json!([{"action":"inspect_scene_actor","level":"manoir","child_path":[0,2],"object_types":[2,2,2],"include_camera_records":true}]);
        assert!(validate_steps(&default_camera).is_ok());
        let searched_camera =
            json!([{"action":"inspect_scene_camera","level":"manoir","object_types":[2,2,2]}]);
        assert!(validate_steps(&searched_camera).is_ok());
        for invalid in [
            json!([{"action":"inspect_scene_camera","level":"manoir","object_types":[2,2,2],"child_path":[0,2]}]),
            json!([{"action":"inspect_scene_actor","level":"manoir","child_path":[0,2],"object_types":[2,2,2],"actions":[]}]),
            json!([{"action":"inspect_scene_actor","level":"manoir","child_path":[],"object_types":[39,62,180],"actions":[0]}]),
            json!([{"action":"inspect_scene_actor","level":"manoir","child_path":[0,100000],"object_types":[39,62,180],"actions":[0]}]),
            json!([{"action":"inspect_scene_actor","level":"manoir","child_path":[0],"object_types":[39,62],"actions":[0]}]),
            json!([{"action":"inspect_scene_actor","level":"manoir","child_path":[0,2],"object_types":[2,2,2],"actions":[0],"include_camera_records":"yes"}]),
        ] {
            assert!(validate_steps(&invalid).is_err());
        }
    }
    #[test]
    fn designer_value_inspection_is_opt_in_bounded_and_prevalidated() {
        assert_eq!(inspect_designer_values(&Map::new()).unwrap(), None);
        let empty_value = json!({"designer_values":[]});
        let empty = object_args(&empty_value).unwrap();
        assert_eq!(inspect_designer_values(empty).unwrap(), Some(vec![]));
        let selected_value = json!({"designer_values":[0,56,176]});
        let selected = object_args(&selected_value).unwrap();
        assert_eq!(
            inspect_designer_values(selected).unwrap(),
            Some(vec![0, 56, 176])
        );
        for invalid in [
            json!({"designer_values":"0"}),
            json!({"designer_values":[1,1]}),
            json!({"designer_values":[4294967296_u64]}),
            json!({"designer_values":(0..65).collect::<Vec<_>>()}),
        ] {
            assert!(inspect_designer_values(object_args(&invalid).unwrap()).is_err());
        }
        assert!(
            validate_steps(&json!([{
                "action":"inspect_main_actor",
                "actions":[47],
                "designer_values":[0,56]
            }]))
            .is_ok()
        );
        assert!(
            validate_steps(&json!([{
                "action":"inspect_main_actor",
                "designer_values":[2,2]
            }]))
            .is_err()
        );
    }
    #[test]
    fn guarded_write_validation_requires_equal_bounded_values() {
        assert!(validate_packed("hex", "00ff", "00aa").is_ok());
        assert!(validate_packed("hex", "00", "0000").is_err());
        assert!(validate_packed("u8", "256", "0").is_err());
    }
    #[test]
    fn base64_vectors() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"foo"), "Zm9v");
    }
    #[test]
    fn constant_time_comparison_has_exact_semantics() {
        assert!(constant_time_equal(b"secret", b"secret"));
        assert!(!constant_time_equal(b"secret", b"secreu"));
        assert!(!constant_time_equal(b"secret", b"secret-long"));
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn loopback_streamable_http_initializes_acknowledges_notification_and_lists_tools() {
        let token = "0123456789abcdef0123456789abcdef";
        let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        let (_temp, state) = state(port);
        let server_state = state.clone();
        let app = router(state, token);
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let unauthorized = StreamableHttpClientTransport::from_config(
            StreamableHttpClientTransportConfig::with_uri(format!("http://127.0.0.1:{port}/mcp"))
                .auth_header("wrong-token"),
        );
        assert!(().serve(unauthorized).await.is_err());

        let mut bad_origin = std::collections::HashMap::new();
        bad_origin.insert(
            header::ORIGIN,
            axum::http::HeaderValue::from_static("http://evil.invalid"),
        );
        let rejected_origin = StreamableHttpClientTransport::from_config(
            StreamableHttpClientTransportConfig::with_uri(format!("http://127.0.0.1:{port}/mcp"))
                .auth_header(token)
                .custom_headers(bad_origin),
        );
        assert!(().serve(rejected_origin).await.is_err());

        let transport = StreamableHttpClientTransport::from_config(
            StreamableHttpClientTransportConfig::with_uri(format!("http://127.0.0.1:{port}/mcp"))
                .auth_header(token),
        );
        let client = ().serve(transport).await.unwrap();
        let listed = client.peer().list_tools(None).await.unwrap();
        assert_eq!(listed.tools.len(), TOOLS.len());
        assert_eq!(listed.tools[0].name, "status");
        assert!(
            listed
                .tools
                .iter()
                .any(|tool| tool.name == "inspect_main_actor")
        );
        assert_eq!(
            server_state
                .initialized_notifications
                .load(Ordering::Relaxed),
            1,
            "the rmcp client must complete initialize -> initialized before tools/list"
        );
        client.cancel().await.unwrap();
        server.abort();
    }

    #[tokio::test]
    async fn protocol_dispatch_and_security_policy_work_without_network_access() {
        let (_temp, state) = state(18766);
        let handler = OrreryMcp { state };
        assert_eq!(handler.get_info().server_info.name, "Orrery");
        assert_eq!(tool_models().len(), TOOLS.len());
        assert!(constant_time_equal(b"Bearer secret", b"Bearer secret"));
        assert!(!constant_time_equal(b"Bearer secret", b"Bearer wrong"));
    }
}
