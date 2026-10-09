#!/usr/bin/env python3
"""Inspect saved native scene ownership without importing or running Godot.

Expand PackedScene instances, then apply inherited node overrides by path. This
reports serialized structure, not instantiated engine nodes or loading timings.
Only native text scenes with external PackedScene instances are supported.
"""
import argparse
from collections import Counter
from dataclasses import dataclass
import hashlib
import json
from pathlib import Path
import re


ATTRIBUTES = re.compile(r'(\w+)="((?:[^"\\]|\\.)*)"')
INSTANCE = re.compile(r'\binstance=ExtResource\("([^"\\]+)"\)')


@dataclass
class Scene:
    nodes: list
    shape_counts: Counter
    shape_bodies: set
    size: int


def read_scene(path):
    resources, nodes = {}, []
    shape_counts, shape_bodies = Counter(), set()
    shape_type, shape_data = None, []

    def finish_shape():
        if shape_type is not None:
            shape_counts[shape_type] += 1
            digest = hashlib.sha256('\n'.join(shape_data).encode()).hexdigest()
            shape_bodies.add((shape_type, digest))

    with path.open() as source:
        for line in source:
            if line.startswith('['):
                finish_shape()
                shape_type, shape_data = None, []
                attrs = {key: json.loads('"' + value + '"')
                         for key, value in ATTRIBUTES.findall(line)}
                if line.startswith('[ext_resource '):
                    if attrs.get('type') == 'PackedScene':
                        resources[attrs['id']] = attrs['path']
                elif line.startswith('[sub_resource '):
                    if attrs.get('type', '').endswith('Shape3D'):
                        shape_type = attrs['type']
                elif line.startswith('[node '):
                    parent = attrs.get('parent')
                    key = '' if parent is None else (
                        attrs['name'] if parent == '.' else parent + '/' + attrs['name'])
                    instance = INSTANCE.search(line)
                    if 'instance=' in line and instance is None:
                        raise ValueError(f'{path}: unsupported non-external scene instance')
                    nodes.append((key, attrs.get('type'),
                                  instance.group(1) if instance else None))
            elif shape_type is not None and line.strip():
                shape_data.append(line.strip())
    finish_shape()
    if not nodes or nodes[0][0] != '':
        raise ValueError(f'{path}: missing native scene root')
    resolved = []
    for key, kind, instance in nodes:
        if instance is not None and instance not in resources:
            raise ValueError(f'{path}: missing PackedScene resource {instance}')
        resolved.append((key, kind, resources.get(instance)))
    return Scene(resolved, shape_counts, shape_bodies, path.stat().st_size)


def census(project, resource):
    project = project.resolve()
    scenes, expanded, visiting = {}, {}, set()

    def expand(reference):
        if reference in expanded:
            return expanded[reference]
        if reference in visiting:
            raise ValueError(f'cyclic PackedScene dependency: {reference}')
        if not reference.startswith('res://') or not reference.endswith('.tscn'):
            raise ValueError(f'unsupported scene reference: {reference}')
        path = (project / reference.removeprefix('res://')).resolve()
        if not path.is_relative_to(project):
            raise ValueError(f'scene escapes project: {reference}')
        visiting.add(reference)
        scene = read_scene(path)
        scenes[reference] = scene
        nodes = {}
        for key, kind, instance in scene.nodes:
            if instance is not None:
                for child, child_kind in expand(instance).items():
                    target = key + '/' + child if key and child else key or child
                    nodes[target] = child_kind
            if kind is not None:
                nodes[key] = kind
            elif key not in nodes:
                raise ValueError(f'{reference}: override has no inherited node: {key}')
        for key in nodes:
            if key and key.rpartition('/')[0] not in nodes:
                raise ValueError(f'{reference}: saved parent is missing: {key}')
        visiting.remove(reference)
        expanded[reference] = nodes
        return nodes

    nodes = expand(resource)
    parked = {key: kind for key, kind in nodes.items()
              if any(part.startswith('bank_') for part in key.split('/'))}
    shape_counts, shape_bodies = Counter(), set()
    for scene in scenes.values():
        shape_counts.update(scene.shape_counts)
        shape_bodies.update(scene.shape_bodies)
    return {
        'entry': resource,
        'scope': 'saved text-scene instance expansion; runtime nodes and timings unmeasured',
        'scene_dependency_count': len(scenes),
        'scene_dependency_bytes': sum(scene.size for scene in scenes.values()),
        'entry_serialized_node_headers': len(scenes[resource].nodes),
        'entry_distinct_serialized_node_paths': len({row[0] for row in scenes[resource].nodes}),
        'distinct_saved_node_paths': len(nodes),
        'node_types': dict(Counter(nodes.values())),
        'parked_bank_node_paths': len(parked),
        'parked_bank_node_types': dict(Counter(parked.values())),
        'embedded_shape_resources': dict(shape_counts),
        'distinct_embedded_shape_bodies': dict(Counter(kind for kind, _ in shape_bodies)),
        'scene_dependencies': sorted(scenes),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('project', type=Path)
    parser.add_argument('scene', help='res:// path to the saved entry scene')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    report = census(args.project, args.scene)
    text = json.dumps(report, indent=2) + '\n'
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(text)
        # Keep the dependency list in the receipt, not in routine terminal output.
        print(json.dumps({key: value for key, value in report.items()
                          if key != 'scene_dependencies'}, indent=2))
    else:
        print(text, end='')


if __name__ == '__main__':
    main()
