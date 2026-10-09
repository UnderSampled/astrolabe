"""Synthetic static inspection checks; never launch Godot or read game data."""
from pathlib import Path
import tempfile
import unittest

from scene_loading_census import census


class SceneLoadingCensusTests(unittest.TestCase):
    def test_nested_instances_and_inherited_overrides_count_each_path_once(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'model.tscn').write_text('''[gd_scene format=3]
[node name="Model" type="MeshInstance3D"]
[node name="Detail" type="Sprite3D" parent="."]
''')
            (root / 'actor.tscn').write_text('''[gd_scene format=3]
[ext_resource type="PackedScene" path="res://model.tscn" id="model"]
[sub_resource type="SphereShape3D" id="one"]
radius = 2.0
[sub_resource type="SphereShape3D" id="two"]
radius = 2.0
[node name="Actor" type="CharacterBody3D"]
[node name="bank_normal" type="Node3D" parent="."]
[node name="payload_0" type="Node3D" parent="bank_normal"]
[node name="Visual" parent="bank_normal/payload_0" instance=ExtResource("model")]
[node name="Visual" parent="bank_normal/payload_0"]
''')
            (root / 'level.tscn').write_text('''[gd_scene format=3]
[ext_resource type="PackedScene" path="res://actor.tscn" id="actor"]
[node name="Level" type="Node3D"]
[node name="First" parent="." instance=ExtResource("actor")]
[node name="Visual" parent="First/bank_normal/payload_0"]
[node name="Second" parent="." instance=ExtResource("actor")]
''')
            before = {p.name: p.read_bytes() for p in root.iterdir()}
            report = census(root, 'res://level.tscn')
            self.assertEqual(report['scene_dependency_count'], 3)
            self.assertEqual(report['distinct_saved_node_paths'], 11)
            self.assertEqual(report['node_types']['MeshInstance3D'], 2)
            self.assertEqual(report['node_types']['Sprite3D'], 2)
            self.assertEqual(report['parked_bank_node_paths'], 8)
            self.assertEqual(report['embedded_shape_resources'], {'SphereShape3D': 2})
            self.assertEqual(report['distinct_embedded_shape_bodies'], {'SphereShape3D': 1})
            self.assertEqual(before, {p.name: p.read_bytes() for p in root.iterdir()})

    def test_missing_inherited_node_and_cyclic_dependency_are_explicit_errors(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            path = root / 'invalid.tscn'
            path.write_text('''[gd_scene format=3]
[node name="Root" type="Node3D"]
[node name="Missing" parent="."]
''')
            with self.assertRaisesRegex(ValueError, 'override has no inherited node'):
                census(root, 'res://invalid.tscn')
            path.write_text('''[gd_scene format=3]
[ext_resource type="PackedScene" path="res://invalid.tscn" id="self"]
[node name="Root" type="Node3D"]
[node name="Child" parent="." instance=ExtResource("self")]
''')
            with self.assertRaisesRegex(ValueError, 'cyclic PackedScene dependency'):
                census(root, 'res://invalid.tscn')

    def test_unsupported_scene_representation_is_not_counted_as_complete(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'binary.tscn').write_text('''[gd_scene format=3]
[ext_resource type="PackedScene" path="res://model.scn" id="model"]
[node name="Root" instance=ExtResource("model")]
''')
            with self.assertRaisesRegex(ValueError, 'unsupported scene reference'):
                census(root, 'res://binary.tscn')


if __name__ == '__main__':
    unittest.main()
