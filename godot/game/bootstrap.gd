extends Control
## Source-free entry point. Loading a converted PackedScene requires rehydration.
const PREVIEW := "res://generated/hype/levels/astrolabe/preview.tscn"

func _ready() -> void:
    var panel := VBoxContainer.new()
    panel.position = Vector2(40, 40)
    add_child(panel)
    var heading := Label.new()
    heading.text = "Astrolabe · Godot development project"
    panel.add_child(heading)
    var details := Label.new()
    details.text = "Support and synthetic examples work without game data.\nSee README.md to generate the supported Hype scene."
    panel.add_child(details)
    var example := Button.new()
    example.text = "Open synthetic example"
    example.pressed.connect(func(): get_tree().change_scene_to_file("res://tests/example.tscn"))
    panel.add_child(example)
    var level := Button.new()
    level.text = "Inspect converted Astrolabe" if ResourceLoader.exists(PREVIEW) else "Astrolabe content has not been generated"
    level.disabled = not ResourceLoader.exists(PREVIEW)
    level.pressed.connect(func(): get_tree().change_scene_to_file(PREVIEW))
    panel.add_child(level)
