# Disposable resource builder. Rust appends statements inside build().
extends SceneTree

func _initialize():
    call_deferred("build")

func build():
    var evidence = {"models": [], "sounds": [], "textures": []}
