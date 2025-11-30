"""OpenSpace engine format parsers."""

from .sna import SNAFile, SNAMemoryBlock, SNAReader, read_sna_file
from .relocation import RelocationTable, RelocationPointerList, RelocationPointerInfo
from .pointer import Pointer, FileWithPointers
from .geometry import (
    GeometricObject,
    GeometricObjectElement,
    GeometricObjectElementTriangles,
    Vector3,
    Vector2,
    Triangle,
)
from .encryption import XORMask, EncryptedReader

__all__ = [
    "SNAFile",
    "SNAMemoryBlock",
    "SNAReader",
    "read_sna_file",
    "RelocationTable",
    "RelocationPointerList",
    "RelocationPointerInfo",
    "Pointer",
    "FileWithPointers",
    "GeometricObject",
    "GeometricObjectElement",
    "GeometricObjectElementTriangles",
    "Vector3",
    "Vector2",
    "Triangle",
    "XORMask",
    "EncryptedReader",
]
