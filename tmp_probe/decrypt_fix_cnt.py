#!/usr/bin/env python3
"""Decrypt fix.cnt catalog per FUN_0044f040 decompilation."""
import struct
import sys
from pathlib import Path

def decrypt_fix_cnt(path: Path):
    data = path.read_bytes()
    enc_size = struct.unpack_from('<I', data, 0)[0]
    size = enc_size ^ 0xAEEA0AE0
    key = [0xE0, 0x0A, 0xEA, 0xAE]

    mem = bytearray(size)
    file_rest = data[4 : 4 + (size - 4)]
    mem[4 : 4 + len(file_rest)] = file_rest
    for i in range(size):
        mem[i] ^= key[i % 4]

    dir_count = struct.unpack_from('<H', mem, 4)[0]
    file_count = struct.unpack_from('<H', mem, 6)[0]

    pos = 8
    dirs = []
    for _ in range(dir_count):
        ln = mem[pos]
        pos += 1
        name = mem[pos : pos + ln].decode('cp1252', errors='replace')
        pos += ln
        dirs.append(name)

    files = []
    for _ in range(file_count):
        idx = struct.unpack_from('<H', mem, pos)[0]
        pos += 2
        ln = mem[pos]
        pos += 1
        name = mem[pos : pos + ln].decode('cp1252', errors='replace')
        pos += ln
        file_offset = struct.unpack_from('<I', mem, pos)[0]
        pos += 4
        files.append({'index': idx, 'name': name, 'offset': file_offset})

    return {
        'file_size': len(data),
        'payload_size': size,
        'dir_count': dir_count,
        'file_count': file_count,
        'directories': dirs,
        'files': files,
    }


def main():
    path = Path(sys.argv[1] if len(sys.argv) > 1 else 'disc/Gamedata/World/Levels/fix.cnt')
    info = decrypt_fix_cnt(path)
    print(f"fix.cnt: {path}")
    print(f"File size: {info['file_size']:,} bytes")
    print(f"Decrypted catalog size: {info['payload_size']:,} bytes")
    print(f"Directories: {info['dir_count']}")
    print(f"Files: {info['file_count']}")
    if info['directories']:
        print("\nDirectories:")
        for i, d in enumerate(info['directories'][:20]):
            print(f"  [{i}] {d}")
        if len(info['directories']) > 20:
            print(f"  ... and {len(info['directories']) - 20} more")
    print("\nFiles (first 30):")
    for f in info['files'][:30]:
        print(f"  [{f['index']:4}] {f['name']} (offset 0x{f['offset']:X})")
    if len(info['files']) > 30:
        print(f"  ... and {len(info['files']) - 30} more")

    # Unique directory prefixes from names
    prefixes = sorted({f['name'].split('\\')[0] if '\\' in f['name'] else '(root)' for f in info['files']})
    print(f"\nName prefixes ({len(prefixes)}): {', '.join(prefixes[:15])}" + (" ..." if len(prefixes) > 15 else ""))


if __name__ == '__main__':
    main()