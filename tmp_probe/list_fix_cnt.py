#!/usr/bin/env python3
"""List fix.cnt contents using CNT header layout from CntReader.cs."""
import struct
import sys
from pathlib import Path

def read_cnt(path: Path):
    data = path.read_bytes()
    pos = 0
    dir_count, file_count = struct.unpack_from('<ii', data, pos)
    pos += 8
    is_xor = data[pos]; pos += 1
    has_checksum = data[pos]; pos += 1
    xor_key = data[pos]; pos += 1

    directories = []
    for _ in range(dir_count):
        slen = struct.unpack_from('<i', data, pos)[0]; pos += 4
        name_bytes = bytearray(data[pos:pos+slen]); pos += slen
        if is_xor:
            for i in range(len(name_bytes)):
                name_bytes[i] ^= xor_key
        directories.append(name_bytes.decode('cp1252', errors='replace').rstrip('\0'))

    if has_checksum and dir_count > 0:
        pos += 1

    files = []
    for _ in range(file_count):
        dir_idx = struct.unpack_from('<i', data, pos)[0]; pos += 4
        flen = struct.unpack_from('<i', data, pos)[0]; pos += 4
        name_bytes = bytearray(data[pos:pos+flen]); pos += flen
        if is_xor:
            for i in range(len(name_bytes)):
                name_bytes[i] ^= xor_key
        filename = name_bytes.decode('cp1252', errors='replace').rstrip('\0')
        file_xor = data[pos:pos+4]; pos += 4
        checksum = struct.unpack_from('<I', data, pos)[0]; pos += 4
        file_ptr = struct.unpack_from('<i', data, pos)[0]; pos += 4
        file_size = struct.unpack_from('<i', data, pos)[0]; pos += 4
        if 0 <= dir_idx < len(directories):
            full = f"{directories[dir_idx]}\\{filename}" if directories[dir_idx] else filename
        else:
            full = filename
        files.append((full.replace('\\', '/'), file_size))

    return {
        'dir_count': dir_count,
        'file_count': file_count,
        'xor': bool(is_xor),
        'checksum': bool(has_checksum),
        'directories': directories,
        'files': files,
    }

def main():
    path = Path(sys.argv[1] if len(sys.argv) > 1 else 'disc/Gamedata/World/Levels/fix.cnt')
    info = read_cnt(path)
    print(f"CNT: {path}")
    print(f"Directories: {info['dir_count']}")
    print(f"Files: {info['file_count']}")
    print(f"XOR: {info['xor']}, Checksum: {info['checksum']}")
    if info['directories']:
        print("Directories:")
        for i, d in enumerate(info['directories'][:20]):
            print(f"  [{i}] {d}")
    print("Files (first 30):")
    for full, size in info['files'][:30]:
        print(f"  {full} ({size} bytes)")
    if len(info['files']) > 30:
        print(f"  ... and {len(info['files']) - 30} more")
    total = sum(s for _, s in info['files'])
    print(f"Total uncompressed payload: {total:,} bytes")

if __name__ == '__main__':
    main()