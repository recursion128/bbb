"""Host-neutral ELF bytes for Linux execution-image tests."""

from __future__ import annotations

import struct


def minimal_elf64(payload: bytes = b"bbb-test-load-image") -> bytes:
    """Build one little-endian x86_64 PT_LOAD image without a host compiler."""

    header_size = 64
    program_header_size = 56
    file_size = header_size + program_header_size + len(payload)
    ident = b"\x7fELF\x02\x01\x01" + b"\0" * 9
    header = struct.pack(
        "<16sHHIQQQIHHHHHH",
        ident,
        2,
        62,
        1,
        0x400078,
        header_size,
        0,
        0,
        header_size,
        program_header_size,
        1,
        0,
        0,
        0,
    )
    program_header = struct.pack(
        "<IIQQQQQQ",
        1,
        5,
        0,
        0x400000,
        0x400000,
        file_size,
        file_size,
        0x1000,
    )
    return header + program_header + payload
