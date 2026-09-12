#!/usr/bin/env python3
"""Test-only independent oracle: liblzo2 safe decoder and Python zlib Adler32.

The manifest names temporary files emitted by Rust tests. This helper neither
imports Rust code nor contributes a codec to the production implementation.
"""
import ctypes
import ctypes.util
import hashlib
import json
from pathlib import Path
import struct
import sys
import zlib

MAX_BYTES = 64 * 1024 * 1024


def library():
    name = ctypes.util.find_library("lzo2")
    if not name:
        raise RuntimeError("independent codec tests require the system liblzo2 runtime")
    lib = ctypes.CDLL(name)
    function = lib.lzo1x_decompress_safe
    function.argtypes = [ctypes.c_void_p, ctypes.c_size_t, ctypes.c_void_p,
                         ctypes.POINTER(ctypes.c_size_t), ctypes.c_void_p]
    function.restype = ctypes.c_int
    return name, function


def decode(function, stream, expected):
    assert 0 <= expected <= MAX_BYTES, "independent decode exceeds limit"
    source = ctypes.create_string_buffer(stream)
    target = ctypes.create_string_buffer(max(expected, 1))
    length = ctypes.c_size_t(expected)
    status = function(source, len(stream), target, ctypes.byref(length), None)
    return status, target.raw[:length.value], length.value


def envelope(function, data):
    assert len(data) >= 20, "truncated independent envelope"
    flag, stored_len, stored_adler, raw_len, raw_adler = struct.unpack_from("<IIIII", data)
    assert flag in (0, 1), f"unknown compression mode {flag}"
    assert len(data) == 20 + stored_len, "independent stored length mismatch"
    stored = data[20:]
    assert zlib.adler32(stored) == stored_adler, "independent stored Adler32 mismatch"
    if flag:
        status, raw, length = decode(function, stored, raw_len)
        assert status == 0, f"liblzo2 rejects envelope stream: {status}"
        assert length == raw_len, "independent decoded length mismatch"
    else:
        assert raw_len == stored_len, "uncompressed lengths differ"
        raw = stored
    assert zlib.adler32(raw) == raw_adler, "independent decoded Adler32 mismatch"
    return raw


def read(path):
    path = Path(path)
    assert path.stat().st_size <= MAX_BYTES, "oracle fixture exceeds size limit"
    return path.read_bytes()


def main(manifest):
    name, function = library()
    valid_empty = decode(function, bytes([0x11, 0, 0]), 0)
    invalid_empty = decode(function, bytes([0x11, 0x11, 0, 0]), 0)
    assert valid_empty == (0, b"", 0), "standard LZO empty terminator failed"
    assert invalid_empty[0] != 0, "liblzo2 accepted the lzokay 2.0.1 empty-stream regression"
    report = {"library": name, "cases": 0, "decoded_bytes": 0,
              "original_envelopes": 0, "changed_compression": 0,
              "empty_regression_status": invalid_empty[0]}
    digest = hashlib.sha256()
    for case in json.loads(Path(manifest).read_text()):
        label = case["label"]
        raw = read(case["raw"])
        compressed = read(case["compressed"])
        status, actual, length = decode(function, compressed, len(raw))
        assert status == 0, f"{label}: liblzo2 rejects new stream: {status}"
        assert length == len(raw) and actual == raw, f"{label}: new stream differs from raw data"
        assert envelope(function, read(case["envelope"])) == raw, f"{label}: new envelope differs"
        if "original_envelope" in case:
            original = read(case["original_envelope"])
            assert envelope(function, original) == raw, f"{label}: original stream decodes differently"
            report["original_envelopes"] += 1
            if original[20:] != compressed:
                report["changed_compression"] += 1
        digest.update(raw)
        report["cases"] += 1
        report["decoded_bytes"] += len(raw)
    report["decoded_sha256"] = digest.hexdigest()
    print(json.dumps(report, sort_keys=True))


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("usage: check_lzo.py MANIFEST.json")
    main(sys.argv[1])
