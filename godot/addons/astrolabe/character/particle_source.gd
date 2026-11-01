extends Node3D
## Actor-local selection of converted native emitter scenes. No source pointers,
## opcode tables, OpenSpace allocation or scheduler state enter this component.
var selected: CPUParticles3D
var enabled := false
var generators: Dictionary = {}

func select_generator(template: PackedScene) -> bool:
    if template == null:
        return false
    var candidate := generators.get(template) as CPUParticles3D
    if candidate == null:
        var instance := template.instantiate()
        candidate = instance as CPUParticles3D
        if candidate == null or not candidate.has_method("initialize_selection"):
            instance.free()
            return false
        if not candidate.call("initialize_selection"):
            candidate.free()
            return false
        add_child(candidate)
        generators[template] = candidate
    if selected != null and selected != candidate:
        selected.emitting = false
    selected = candidate
    return true

func set_generator_enabled(requested: bool) -> bool:
    if selected == null:
        # Turning off a generator that was never selected changes nothing.
        enabled = false
        return not requested
    if requested and not selected.call("enable_source_emission"):
        return false
    enabled = requested
    selected.emitting = requested
    return true
