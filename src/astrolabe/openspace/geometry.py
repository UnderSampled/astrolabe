"""Geometry data structures for OpenSpace Montreal engine.

This module parses GeometricObject and related mesh data from SNA blocks.
"""

from dataclasses import dataclass, field
import struct
from typing import TYPE_CHECKING, Optional

if TYPE_CHECKING:
    from .pointer import SNADataReader


@dataclass
class Vector3:
    """3D vector."""

    x: float
    y: float
    z: float

    def to_tuple(self) -> tuple[float, float, float]:
        """Convert to tuple with Y/Z swapped for glTF coordinate system."""
        # OpenSpace uses X-right, Z-up, Y-forward
        # glTF uses X-right, Y-up, Z-forward
        return (self.x, self.z, self.y)


@dataclass
class Vector2:
    """2D vector (UV coordinates)."""

    u: float
    v: float

    def to_tuple(self) -> tuple[float, float]:
        """Convert to tuple, flipping V for glTF."""
        return (self.u, 1.0 - self.v)


@dataclass
class Triangle:
    """Triangle indices."""

    v0: int
    v1: int
    v2: int


@dataclass
class GeometricObjectElement:
    """Base class for mesh elements (submeshes)."""

    element_type: int
    offset: int


@dataclass
class GeometricObjectElementTriangles(GeometricObjectElement):
    """Triangle mesh element (submesh)."""

    material_offset: int = 0
    num_triangles: int = 0
    num_uvs: int = 0
    num_uv_maps: int = 1
    triangles: list[Triangle] = field(default_factory=list)
    uvs: list[Vector2] = field(default_factory=list)
    uv_mapping: list[list[int]] = field(default_factory=list)  # Per UV map


@dataclass
class GeometricObject:
    """A complete geometric object (mesh) from OpenSpace.

    Contains vertices, normals, and submesh elements.
    """

    offset: int
    num_vertices: int = 0
    num_elements: int = 0
    vertices: list[Vector3] = field(default_factory=list)
    normals: list[Vector3] = field(default_factory=list)
    elements: list[GeometricObjectElement] = field(default_factory=list)
    element_types: list[int] = field(default_factory=list)
    sphere_center: Vector3 = field(default_factory=lambda: Vector3(0, 0, 0))
    sphere_radius: float = 0.0
    look_at_mode: int = 0


def read_geometric_object(reader: "SNADataReader", offset: int) -> GeometricObject:
    """Read a GeometricObject from the data stream.

    Args:
        reader: Data reader positioned at the object.
        offset: Offset of the object for reference.

    Returns:
        Parsed GeometricObject.
    """
    geo = GeometricObject(offset=offset)

    # Montreal engine format:
    # u32 num_vertices
    # ptr off_vertices
    # ptr off_normals
    # ptr off_materials
    # u32 unk
    # u32 num_elements
    # ptr off_element_types
    # ptr off_elements
    # ... more fields

    geo.num_vertices = reader.read_u32()
    off_vertices = reader.read_pointer()
    off_normals = reader.read_pointer()
    off_materials = reader.read_pointer()
    reader.skip(4)  # Unknown
    geo.num_elements = reader.read_u32()
    off_element_types = reader.read_pointer()
    off_elements = reader.read_pointer()

    # Skip some unknown fields
    reader.skip(4)  # unk
    reader.skip(4)  # off_parallel_boxes or unk
    reader.skip(4)  # unk
    reader.skip(4)  # unk

    # Bounding sphere
    geo.sphere_radius = reader.read_float()
    sphere_x = reader.read_float()
    sphere_z = reader.read_float()
    sphere_y = reader.read_float()
    geo.sphere_center = Vector3(sphere_x, sphere_y, sphere_z)

    # Read vertices
    if not off_vertices.is_null and geo.num_vertices > 0:
        saved_pos = reader.tell()
        reader.seek(off_vertices.offset)
        for _ in range(geo.num_vertices):
            x = reader.read_float()
            z = reader.read_float()
            y = reader.read_float()
            geo.vertices.append(Vector3(x, y, z))
        reader.seek(saved_pos + reader.base_address)

    # Read normals
    if not off_normals.is_null and geo.num_vertices > 0:
        saved_pos = reader.tell()
        reader.seek(off_normals.offset)
        for _ in range(geo.num_vertices):
            x = reader.read_float()
            z = reader.read_float()
            y = reader.read_float()
            geo.normals.append(Vector3(x, y, z))
        reader.seek(saved_pos + reader.base_address)

    # Read element types
    if not off_element_types.is_null and geo.num_elements > 0:
        saved_pos = reader.tell()
        reader.seek(off_element_types.offset)
        for _ in range(geo.num_elements):
            geo.element_types.append(reader.read_u16())
        reader.seek(saved_pos + reader.base_address)

    # Read elements (simplified - just read triangle elements)
    if not off_elements.is_null and geo.num_elements > 0:
        for i in range(geo.num_elements):
            # Read element pointer from element array
            reader.seek(off_elements.offset + (i * 4))
            element_ptr = reader.read_pointer()

            if element_ptr.is_null:
                continue

            element_type = geo.element_types[i] if i < len(geo.element_types) else 0

            if element_type == 1:  # Triangle mesh
                reader.seek(element_ptr.offset)
                element = read_triangle_element(reader, element_ptr.offset, geo)
                geo.elements.append(element)
            else:
                # Other element types (sprites, bones, etc.) - skip for now
                geo.elements.append(
                    GeometricObjectElement(element_type=element_type, offset=element_ptr.offset)
                )

    return geo


def read_triangle_element(
    reader: "SNADataReader", offset: int, geo: GeometricObject
) -> GeometricObjectElementTriangles:
    """Read a triangle element (submesh).

    Args:
        reader: Data reader positioned at the element.
        offset: Offset of the element.
        geo: Parent geometric object.

    Returns:
        Parsed triangle element.
    """
    element = GeometricObjectElementTriangles(element_type=1, offset=offset)

    # Montreal format:
    # ptr off_material
    # u16 num_triangles
    # u16 num_uvs
    # ptr off_triangles
    # ptr off_mapping_uvs
    # ptr off_normals
    # ptr off_uvs
    # u32 unk
    # ptr off_vertex_indices
    # u16 num_vertex_indices
    # u16 parallel_box
    # u32 unk

    element.material_offset = reader.read_pointer().offset
    element.num_triangles = reader.read_u16()
    element.num_uvs = reader.read_u16()

    off_triangles = reader.read_pointer()
    off_mapping_uvs = reader.read_pointer()
    off_normals = reader.read_pointer()
    off_uvs = reader.read_pointer()

    reader.skip(4)  # unk

    off_vertex_indices = reader.read_pointer()
    num_vertex_indices = reader.read_u16()
    parallel_box = reader.read_u16()
    reader.skip(4)  # unk

    # Default to 1 UV map for Montreal engine
    element.num_uv_maps = 1

    # Read triangles
    if not off_triangles.is_null and element.num_triangles > 0:
        saved_pos = reader.tell()
        reader.seek(off_triangles.offset)
        for _ in range(element.num_triangles):
            v0 = reader.read_i16()
            v1 = reader.read_i16()
            v2 = reader.read_i16()
            element.triangles.append(Triangle(v0, v1, v2))
        reader.seek(saved_pos + reader.base_address)

    # Read UVs
    if not off_uvs.is_null and element.num_uvs > 0:
        saved_pos = reader.tell()
        reader.seek(off_uvs.offset)
        for _ in range(element.num_uvs):
            u = reader.read_float()
            v = reader.read_float()
            element.uvs.append(Vector2(u, v))
        reader.seek(saved_pos + reader.base_address)

    # Read UV mapping (which UV index for each triangle vertex)
    if not off_mapping_uvs.is_null and element.num_triangles > 0:
        saved_pos = reader.tell()
        reader.seek(off_mapping_uvs.offset)
        uv_map = []
        for _ in range(element.num_triangles * 3):
            uv_map.append(reader.read_i16())
        element.uv_mapping.append(uv_map)
        reader.seek(saved_pos + reader.base_address)

    return element
