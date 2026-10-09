extends RefCounted
## Loading owns the readiness barrier; each compiled Brain still owns its
## behavior and every character still owns its activation/physics policy.
const BEHAVIOR = preload("res://addons/astrolabe/behavior/behavior.gd")
const CHARACTER = preload("res://addons/astrolabe/character/character.gd")

var brains: Array[Dictionary] = []
var bodies: Array[Dictionary] = []


func hold_before_tree(level: Node3D) -> String:
    if level == null or level.is_inside_tree() or not brains.is_empty():
        return "Saved-level startup barrier requires a fresh unattached level."
    for candidate in level.find_children("Brain", "Node", true, false):
        if not candidate is BEHAVIOR:
            return "Saved level has an incompatible Brain: " + str(level.get_path_to(candidate))
        if candidate.running:
            return "Saved level has a Brain running before restoration."
    for candidate in level.find_children("Brain", "Node", true, false):
        brains.append({"brain": candidate, "automatic": candidate.automatic})
        candidate.automatic = false
    for candidate in level.find_children("*", "CharacterBody3D", true, false):
        if candidate is CHARACTER: bodies.append({"body": candidate})
    return ""


func hold_after_tree() -> String:
    # Character _ready establishes its native activation state. Capture that
    # result, then prevent the first motion tick from observing partial state.
    for state in brains:
        var brain: BEHAVIOR = state["brain"]
        if not is_instance_valid(brain) or not brain.is_inside_tree() or brain.running:
            return "A saved Brain started before restoration completed."
    for state in bodies:
        var body: CHARACTER = state["body"]
        if not is_instance_valid(body) or not body.is_inside_tree():
            return "A saved character is unavailable during restoration."
        state["physics"] = body.is_physics_processing()
        body.set_physics_process(false)
    return ""


func release_physics() -> void:
    for state in bodies:
        var body: CHARACTER = state["body"]
        if is_instance_valid(body) and not body.is_queued_for_deletion():
            # Save restoration can consume/deactivate an actor. Preserve that
            # component state rather than resurrecting its pre-restore process.
            body.set_physics_process(bool(state.get("physics", false)) and body.source_object_active and not body.source_return_suppressed)


func start_brains() -> String:
    for state in brains:
        var brain: BEHAVIOR = state["brain"]
        if not is_instance_valid(brain) or brain.is_queued_for_deletion(): continue
        brain.automatic = state["automatic"]
        # Earlier owner initialization may legitimately request a behavior on
        # a later actor; preserve it without restarting its native continuation.
        # Match behavior._ready's eligibility: unbound pooled definitions are
        # started when their owning generated actor acquires a MainActor.
        if brain.automatic and brain.main_actor != null and not brain.running and not brain.start():
            return "Saved compiled behavior could not start: " + str(brain.get_parent().get_meta("source_perso", ""))
    return ""
