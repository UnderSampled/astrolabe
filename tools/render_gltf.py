#!/usr/bin/env python3
"""
Render a GLTF file to PNG using Blender.

Usage:
    flatpak run org.blender.Blender --background --python render_gltf.py -- input.glb output.png
"""

import bpy
import sys
import math
import os
from mathutils import Vector

def clear_scene():
    """Remove all objects from the scene."""
    bpy.ops.object.select_all(action='SELECT')
    bpy.ops.object.delete()

    # Also clear collections
    for collection in bpy.data.collections:
        bpy.data.collections.remove(collection)

def import_gltf(filepath):
    """Import a GLTF/GLB file."""
    bpy.ops.import_scene.gltf(filepath=filepath)

def setup_camera_to_fit_scene():
    """Position camera to fit all objects in view."""
    # Get bounding box of all mesh objects
    min_coords = [float('inf')] * 3
    max_coords = [float('-inf')] * 3

    has_objects = False
    for obj in bpy.context.scene.objects:
        if obj.type == 'MESH':
            has_objects = True
            for corner in obj.bound_box:
                world_corner = obj.matrix_world @ Vector(corner)
                for i in range(3):
                    min_coords[i] = min(min_coords[i], world_corner[i])
                    max_coords[i] = max(max_coords[i], world_corner[i])

    if not has_objects:
        print("No mesh objects found!")
        return None

    # Calculate center and size
    center = [(min_coords[i] + max_coords[i]) / 2 for i in range(3)]
    size = [max_coords[i] - min_coords[i] for i in range(3)]
    max_size = max(size) if max(size) > 0 else 1

    print(f"Scene bounds: min={min_coords}, max={max_coords}")
    print(f"Scene center: {center}, size: {size}, max_size: {max_size}")

    # Create camera
    bpy.ops.object.camera_add()
    camera = bpy.context.active_object
    bpy.context.scene.camera = camera

    # Position camera at an angle looking at center
    distance = max_size * 2.5
    camera.location = (
        center[0] + distance * 0.7,
        center[1] - distance * 0.7,
        center[2] + distance * 0.5
    )

    # Point camera at center
    direction = Vector(center) - camera.location
    rot_quat = direction.to_track_quat('-Z', 'Y')
    camera.rotation_euler = rot_quat.to_euler()

    # Adjust camera clip distances
    camera.data.clip_start = 0.1
    camera.data.clip_end = distance * 10

    return camera

def setup_lighting():
    """Add basic lighting to the scene."""
    # Add sun light
    bpy.ops.object.light_add(type='SUN', location=(10, -10, 20))
    sun = bpy.context.active_object
    sun.data.energy = 3.0
    sun.rotation_euler = (math.radians(45), math.radians(30), math.radians(45))

    # Add fill light
    bpy.ops.object.light_add(type='POINT', location=(-10, 10, 10))
    fill = bpy.context.active_object
    fill.data.energy = 500.0

def setup_render_settings(output_path, resolution=1024):
    """Configure render settings."""
    scene = bpy.context.scene

    # Use Cycles for better quality, or EEVEE for speed
    scene.render.engine = 'BLENDER_EEVEE_NEXT' if hasattr(bpy.types, 'BLENDER_EEVEE_NEXT') else 'BLENDER_EEVEE'

    # Resolution
    scene.render.resolution_x = resolution
    scene.render.resolution_y = resolution
    scene.render.resolution_percentage = 100

    # Output
    scene.render.filepath = output_path
    scene.render.image_settings.file_format = 'PNG'

    # Background
    scene.world = bpy.data.worlds.new("World")
    scene.world.use_nodes = True
    bg_node = scene.world.node_tree.nodes.get('Background')
    if bg_node:
        bg_node.inputs[0].default_value = (0.1, 0.1, 0.15, 1)  # Dark blue-gray

def render(output_path):
    """Render the scene to an image."""
    bpy.ops.render.render(write_still=True)
    print(f"Rendered to: {output_path}")

def main():
    # Parse arguments after "--"
    argv = sys.argv
    if "--" in argv:
        argv = argv[argv.index("--") + 1:]
    else:
        print("Usage: blender --background --python render_gltf.py -- input.glb output.png")
        sys.exit(1)

    if len(argv) < 2:
        print("Error: Need input.glb and output.png arguments")
        sys.exit(1)

    input_path = os.path.abspath(argv[0])
    output_path = os.path.abspath(argv[1])

    print(f"Input: {input_path}")
    print(f"Output: {output_path}")

    # Clear and import
    clear_scene()
    import_gltf(input_path)

    # Count imported objects
    mesh_count = sum(1 for obj in bpy.context.scene.objects if obj.type == 'MESH')
    print(f"Imported {mesh_count} mesh objects")

    # Setup scene
    camera = setup_camera_to_fit_scene()
    if camera is None:
        print("Failed to setup camera - no meshes found")
        sys.exit(1)

    setup_lighting()
    setup_render_settings(output_path)

    # Render
    render(output_path)

if __name__ == "__main__":
    main()
