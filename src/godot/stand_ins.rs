//! Documented guesses for source AI calls the port cannot yet perform.
//!
//! A stand-in keeps the game running in place of a call whose real behavior is
//! not implemented. It is never a silent no-op: compiled code records it through
//! `stand_in()`, each entry states the guess and what game-accurate behavior
//! replaces it, and strict Brains (the test default) still stop at it. See
//! docs/astrolabe/stand-ins.md, which must list every id here.

pub(super) struct StandIn {
    /// `<category>-<operation>` as printed by the AI decoder's reason text.
    pub id: &'static str,
    pub name: &'static str,
    /// GDScript value for expressions; unused by statements.
    pub value: &'static str,
    pub guess: &'static str,
}

const fn entry(
    id: &'static str,
    name: &'static str,
    value: &'static str,
    guess: &'static str,
) -> StandIn {
    StandIn {
        id,
        name,
        value,
        guess,
    }
}

const KEEP_CAMERA: &str = "keep the native follow camera's current tuning";

pub(super) const STAND_INS: &[StandIn] = &[
    entry(
        "3-189",
        "MTH_Modulus",
        "1",
        "report a nonzero remainder, so periodic player life and mana regeneration does not tick",
    ),
    entry(
        "1-65",
        "Cond_IsValidWayPoint",
        "false",
        "treat the waypoint as invalid",
    ),
    entry(
        "1-118",
        "Cond_Inv_InventoryIsFull",
        "false",
        "the inventory always has room",
    ),
    entry(
        "1-119",
        "Cond_Inv_FindObject",
        "false",
        "the object is not carried",
    ),
    entry(
        "1-32",
        "Cond_CollideWithWall",
        "false",
        "no wall contact is reported to the script",
    ),
    entry("3-197", "Func_Inv_RemoveObject", "0", "nothing is removed"),
    entry("3-199", "Func_Inv_AddGold", "0", "no gold is added"),
    entry(
        "4-417",
        "Inv_UnselectSelectedObject",
        "",
        "the selected object stays selected",
    ),
    entry(
        "4-438",
        "Magic_SelectNextSpellType",
        "",
        "the selected spell stays selected",
    ),
    entry(
        "4-334",
        "Proc_ActivateInventoryMotorMode",
        "",
        "the authored Tab host owns the inventory screen",
    ),
    entry(
        "4-374",
        "Proc_ActivateMapMotorMode",
        "",
        "the map screen does not open",
    ),
    entry(
        "4-336",
        "Proc_ActivateSaveGameMotorMode",
        "",
        "the authored F5 save owns saving",
    ),
    entry(
        "4-271",
        "Map_vSetDestination",
        "",
        "the map destination is not recorded",
    ),
    entry(
        "4-272",
        "Map_vResetDestination",
        "",
        "the map destination is not recorded",
    ),
    entry("4-153", "Procedure_Camera_ChangeAngle", "", KEEP_CAMERA),
    entry("4-143", "Procedure_Camera_ChangeDistance", "", KEEP_CAMERA),
    entry("4-154", "Procedure_Camera_Move", "", KEEP_CAMERA),
    entry("4-150", "Procedure_Camera_MoveTarget", "", KEEP_CAMERA),
    entry("4-152", "Procedure_Camera_MoveShift", "", KEEP_CAMERA),
    entry("4-151", "Procedure_Camera_MoveLookTarget", "", KEEP_CAMERA),
    entry(
        "4-175",
        "Procedure_Camera_SetCameraPosition",
        "",
        KEEP_CAMERA,
    ),
    entry(
        "4-148",
        "Procedure_Camera_ChangeCamAngularInertia",
        "",
        KEEP_CAMERA,
    ),
    entry(
        "4-146",
        "Procedure_Camera_ChangeCamLinearInertia",
        "",
        KEEP_CAMERA,
    ),
    entry(
        "4-140",
        "Procedure_Camera_ShowInfo",
        "",
        "debug camera text is not shown",
    ),
    entry(
        "3-60",
        "Function_Camera_GetTeta",
        "0.0",
        "report a level camera",
    ),
    entry(
        "3-58",
        "Function_Camera_GetAlpha",
        "0.0",
        "report a level camera",
    ),
    entry(
        "3-54",
        "Function_Camera_GetDistMin",
        "0.0",
        "report no minimum distance",
    ),
    entry(
        "3-55",
        "Function_Camera_GetDistMax",
        "0.0",
        "report no maximum distance",
    ),
    entry(
        "3-70",
        "Function_Camera_GetState",
        "0",
        "report the default camera state",
    ),
    entry(
        "3-75",
        "Function_Camera_GetLookVertex",
        "Vector3.ZERO",
        "report the origin as the look point",
    ),
];

const WAYPOINT: StandIn = entry(
    "unbound-waypoint",
    "unbound absolute waypoint",
    "Vector3.ZERO",
    "read the level origin as the waypoint",
);

const GENERIC: StandIn = entry(
    "generic",
    "undocumented call",
    "0",
    "skip the call; an expression reads as zero",
);

/// The entry for a decoder reason, and the id compiled code records.
pub(super) fn for_reason(reason: &str) -> (&'static StandIn, String) {
    let operation = reason.split_once("category ").and_then(|(_, tail)| {
        let (category, tail) = tail.split_once(" operation ")?;
        let digits = tail.split(|c: char| !c.is_ascii_digit()).next()?;
        Some(format!("{category}-{digits}"))
    });
    if reason.starts_with("unbound absolute waypoint") {
        return (&WAYPOINT, "unbound-waypoint".into());
    }
    match operation {
        Some(id) => match STAND_INS.iter().find(|entry| entry.id == id) {
            Some(entry) => (entry, id),
            None => (&GENERIC, format!("generic-{id}")),
        },
        None => (&GENERIC, "generic-untyped".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stand_ins_are_found_by_decoder_reason_and_documented() {
        let (entry, id) = for_reason("unsupported AI statement category 4 operation 153 at node 9");
        assert_eq!(
            (entry.name, id.as_str()),
            ("Procedure_Camera_ChangeAngle", "4-153")
        );
        assert_eq!(
            for_reason("unsupported AI statement category 5 operation 7 at node 1").1,
            "generic-5-7"
        );
        assert_eq!(
            for_reason("unbound absolute waypoint position").1,
            "unbound-waypoint"
        );
        let document = include_str!("../../docs/astrolabe/stand-ins.md");
        assert!(document.contains("`unbound-waypoint`"));
        for entry in STAND_INS {
            assert!(
                document.contains(&format!("`{}`", entry.id)),
                "{} is undocumented",
                entry.id
            );
        }
    }
}
