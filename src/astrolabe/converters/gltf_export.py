"""Export OpenSpace geometry to glTF format."""

import struct
from pathlib import Path
from typing import TYPE_CHECKING

import numpy as np
from pygltflib import (
    GLTF2,
    Accessor,
    Asset,
    Attributes,
    Buffer,
    BufferView,
    Mesh,
    Node,
    Primitive,
    Scene,
)

if TYPE_CHECKING:
    from astrolabe.openspace.geometry import GeometricObject


def export_geometry_to_gltf(
    geo: "GeometricObject",
    output_path: Path,
    name: str = "mesh",
) -> None:
    """Export a GeometricObject to glTF format.

    Args:
        geo: The geometric object to export.
        output_path: Path to write the .gltf file.
        name: Name for the mesh.
    """
    # Convert vertices to numpy array
    if not geo.vertices:
        raise ValueError("No vertices in geometry")

    vertices = np.array(
        [v.to_tuple() for v in geo.vertices],
        dtype=np.float32,
    )

    # Convert normals if available
    has_normals = len(geo.normals) == len(geo.vertices)
    if has_normals:
        normals = np.array(
            [n.to_tuple() for n in geo.normals],
            dtype=np.float32,
        )

    # Collect all triangles from all triangle elements
    all_indices: list[int] = []
    all_uvs: list[tuple[float, float]] = []
    uv_indices: list[int] = []

    for element in geo.elements:
        from astrolabe.openspace.geometry import GeometricObjectElementTriangles

        if not isinstance(element, GeometricObjectElementTriangles):
            continue

        for tri in element.triangles:
            all_indices.extend([tri.v0, tri.v1, tri.v2])

        # Collect UVs if available
        if element.uvs and element.uv_mapping:
            for uv in element.uvs:
                all_uvs.append(uv.to_tuple())
            uv_indices.extend(element.uv_mapping[0] if element.uv_mapping else [])

    if not all_indices:
        raise ValueError("No triangles in geometry")

    indices = np.array(all_indices, dtype=np.uint16)

    # Build the binary buffer
    buffer_data = bytearray()

    # Add vertices
    vertex_offset = len(buffer_data)
    buffer_data.extend(vertices.tobytes())
    vertex_byte_length = len(vertices.tobytes())

    # Add normals if available
    if has_normals:
        normal_offset = len(buffer_data)
        buffer_data.extend(normals.tobytes())
        normal_byte_length = len(normals.tobytes())

    # Add indices
    indices_offset = len(buffer_data)
    buffer_data.extend(indices.tobytes())
    indices_byte_length = len(indices.tobytes())

    # Calculate bounds for vertices
    v_min = vertices.min(axis=0).tolist()
    v_max = vertices.max(axis=0).tolist()

    # Create glTF structure
    gltf = GLTF2(
        asset=Asset(version="2.0", generator="Astrolabe"),
        buffers=[
            Buffer(byteLength=len(buffer_data)),
        ],
        bufferViews=[
            # Vertices
            BufferView(
                buffer=0,
                byteOffset=vertex_offset,
                byteLength=vertex_byte_length,
                target=34962,  # ARRAY_BUFFER
            ),
        ],
        accessors=[
            # Vertices
            Accessor(
                bufferView=0,
                byteOffset=0,
                componentType=5126,  # FLOAT
                count=len(vertices),
                type="VEC3",
                min=v_min,
                max=v_max,
            ),
        ],
        meshes=[
            Mesh(
                name=name,
                primitives=[
                    Primitive(
                        attributes=Attributes(POSITION=0),
                        mode=4,  # TRIANGLES
                    ),
                ],
            ),
        ],
        nodes=[
            Node(mesh=0, name=name),
        ],
        scenes=[
            Scene(nodes=[0]),
        ],
        scene=0,
    )

    # Add normals buffer view and accessor
    buffer_view_idx = 1
    accessor_idx = 1

    if has_normals:
        gltf.bufferViews.append(
            BufferView(
                buffer=0,
                byteOffset=normal_offset,
                byteLength=normal_byte_length,
                target=34962,  # ARRAY_BUFFER
            )
        )
        gltf.accessors.append(
            Accessor(
                bufferView=buffer_view_idx,
                byteOffset=0,
                componentType=5126,  # FLOAT
                count=len(normals),
                type="VEC3",
            )
        )
        gltf.meshes[0].primitives[0].attributes.NORMAL = accessor_idx
        buffer_view_idx += 1
        accessor_idx += 1

    # Add indices buffer view and accessor
    gltf.bufferViews.append(
        BufferView(
            buffer=0,
            byteOffset=indices_offset,
            byteLength=indices_byte_length,
            target=34963,  # ELEMENT_ARRAY_BUFFER
        )
    )
    gltf.accessors.append(
        Accessor(
            bufferView=buffer_view_idx,
            byteOffset=0,
            componentType=5123,  # UNSIGNED_SHORT
            count=len(indices),
            type="SCALAR",
        )
    )
    gltf.meshes[0].primitives[0].indices = accessor_idx

    # Set binary blob
    gltf.set_binary_blob(bytes(buffer_data))

    # Save as .glb (binary glTF)
    if output_path.suffix == ".glb":
        gltf.save_binary(str(output_path))
    else:
        # Save as .gltf with embedded buffer
        import base64

        gltf.buffers[0].uri = (
            "data:application/octet-stream;base64,"
            + base64.b64encode(bytes(buffer_data)).decode("ascii")
        )
        gltf.save(str(output_path))


def export_simple_mesh_to_gltf(
    vertices: list[tuple[float, float, float]],
    indices: list[int],
    output_path: Path,
    name: str = "mesh",
    normals: list[tuple[float, float, float]] | None = None,
) -> None:
    """Export a simple mesh (vertices + indices) to glTF.

    Args:
        vertices: List of (x, y, z) vertex positions.
        indices: List of triangle indices (3 per triangle).
        output_path: Path to write the .gltf/.glb file.
        name: Name for the mesh.
        normals: Optional list of (x, y, z) normals per vertex.
    """
    verts_array = np.array(vertices, dtype=np.float32)
    indices_array = np.array(indices, dtype=np.uint16)

    buffer_data = bytearray()

    # Vertices
    vertex_offset = len(buffer_data)
    buffer_data.extend(verts_array.tobytes())
    vertex_byte_length = len(verts_array.tobytes())

    # Normals
    has_normals = normals is not None and len(normals) == len(vertices)
    if has_normals:
        normals_array = np.array(normals, dtype=np.float32)
        normal_offset = len(buffer_data)
        buffer_data.extend(normals_array.tobytes())
        normal_byte_length = len(normals_array.tobytes())

    # Indices
    indices_offset = len(buffer_data)
    buffer_data.extend(indices_array.tobytes())
    indices_byte_length = len(indices_array.tobytes())

    v_min = verts_array.min(axis=0).tolist()
    v_max = verts_array.max(axis=0).tolist()

    gltf = GLTF2(
        asset=Asset(version="2.0", generator="Astrolabe"),
        buffers=[Buffer(byteLength=len(buffer_data))],
        bufferViews=[
            BufferView(
                buffer=0,
                byteOffset=vertex_offset,
                byteLength=vertex_byte_length,
                target=34962,
            ),
        ],
        accessors=[
            Accessor(
                bufferView=0,
                byteOffset=0,
                componentType=5126,
                count=len(verts_array),
                type="VEC3",
                min=v_min,
                max=v_max,
            ),
        ],
        meshes=[
            Mesh(
                name=name,
                primitives=[
                    Primitive(
                        attributes=Attributes(POSITION=0),
                        mode=4,
                    ),
                ],
            ),
        ],
        nodes=[Node(mesh=0, name=name)],
        scenes=[Scene(nodes=[0])],
        scene=0,
    )

    buffer_view_idx = 1
    accessor_idx = 1

    if has_normals:
        gltf.bufferViews.append(
            BufferView(
                buffer=0,
                byteOffset=normal_offset,
                byteLength=normal_byte_length,
                target=34962,
            )
        )
        gltf.accessors.append(
            Accessor(
                bufferView=buffer_view_idx,
                byteOffset=0,
                componentType=5126,
                count=len(normals_array),
                type="VEC3",
            )
        )
        gltf.meshes[0].primitives[0].attributes.NORMAL = accessor_idx
        buffer_view_idx += 1
        accessor_idx += 1

    gltf.bufferViews.append(
        BufferView(
            buffer=0,
            byteOffset=indices_offset,
            byteLength=indices_byte_length,
            target=34963,
        )
    )
    gltf.accessors.append(
        Accessor(
            bufferView=buffer_view_idx,
            byteOffset=0,
            componentType=5123,
            count=len(indices_array),
            type="SCALAR",
        )
    )
    gltf.meshes[0].primitives[0].indices = accessor_idx

    gltf.set_binary_blob(bytes(buffer_data))

    if output_path.suffix == ".glb":
        gltf.save_binary(str(output_path))
    else:
        import base64

        gltf.buffers[0].uri = (
            "data:application/octet-stream;base64,"
            + base64.b64encode(bytes(buffer_data)).decode("ascii")
        )
        gltf.save(str(output_path))
