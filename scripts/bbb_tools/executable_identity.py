"""Stable identities for the runtime-relevant portion of executable files."""

from __future__ import annotations

import hashlib
import struct
from pathlib import Path

ELF_LOAD_IMAGE_ALGORITHM = "elf-load-image-v1"
_ELF_MAGIC = b"\x7fELF"
_ELFCLASS32 = 1
_ELFCLASS64 = 2
_ELFDATA2LSB = 1
_ELFDATA2MSB = 2
_PT_LOAD = 1
_PT_NOTE = 4
_NT_GNU_BUILD_ID = 3


class ExecutableIdentityError(ValueError):
    """The executable cannot provide the requested stable runtime identity."""


def _align(value: int, alignment: int) -> int:
    return (value + alignment - 1) & ~(alignment - 1)


def _zero_file_range(
    segment: bytearray,
    *,
    segment_offset: int,
    start: int,
    size: int,
) -> None:
    overlap_start = max(segment_offset, start)
    overlap_end = min(segment_offset + len(segment), start + size)
    if overlap_start >= overlap_end:
        return
    local_start = overlap_start - segment_offset
    local_end = overlap_end - segment_offset
    segment[local_start:local_end] = b"\0" * (local_end - local_start)


def _gnu_build_id_ranges(
    payload: bytes, note_segments: list[tuple[int, int]], endian: str
) -> list[tuple[int, int]]:
    ranges: list[tuple[int, int]] = []
    note_header = struct.Struct(f"{endian}III")
    for offset, size in note_segments:
        end = offset + size
        cursor = offset
        if offset < 0 or end > len(payload):
            raise ExecutableIdentityError("ELF note segment extends beyond the file")
        while cursor < end:
            if cursor + note_header.size > end:
                raise ExecutableIdentityError("ELF note header is truncated")
            namesz, descsz, note_type = note_header.unpack_from(payload, cursor)
            name_offset = cursor + note_header.size
            desc_offset = _align(name_offset + namesz, 4)
            next_note = _align(desc_offset + descsz, 4)
            if desc_offset > end or next_note > end:
                raise ExecutableIdentityError("ELF note payload is truncated")
            name = payload[name_offset : name_offset + namesz].rstrip(b"\0")
            if name == b"GNU" and note_type == _NT_GNU_BUILD_ID:
                ranges.append((desc_offset, descsz))
            cursor = next_note
    return ranges


def elf_load_image_identity(path: Path) -> dict[str, object]:
    """Hash the Linux ELF execution image while excluding linker-only metadata.

    Section headers, symbol/string tables, and the GNU build-id do not change
    what the kernel and dynamic loader map.  The digest retains the execution
    header, every program header, every PT_LOAD byte, and all other note data.
    """
    payload = path.read_bytes()
    if len(payload) < 16 or payload[:4] != _ELF_MAGIC:
        raise ExecutableIdentityError(f"executable is not ELF: {path}")
    elf_class = payload[4]
    data_encoding = payload[5]
    if data_encoding == _ELFDATA2LSB:
        endian = "<"
    elif data_encoding == _ELFDATA2MSB:
        endian = ">"
    else:
        raise ExecutableIdentityError("ELF byte order is unsupported")

    if elf_class == _ELFCLASS64:
        header = struct.Struct(f"{endian}16sHHIQQQIHHHHHH")
        program_header = struct.Struct(f"{endian}IIQQQQQQ")
        section_metadata_ranges = ((40, 8), (58, 2), (60, 2), (62, 2))
    elif elf_class == _ELFCLASS32:
        header = struct.Struct(f"{endian}16sHHIIIIIHHHHHH")
        program_header = struct.Struct(f"{endian}IIIIIIII")
        section_metadata_ranges = ((32, 4), (46, 2), (48, 2), (50, 2))
    else:
        raise ExecutableIdentityError("ELF class is unsupported")
    if len(payload) < header.size:
        raise ExecutableIdentityError("ELF header is truncated")

    fields = header.unpack_from(payload)
    (
        ident,
        elf_type,
        machine,
        version,
        entry,
        program_offset,
        _section_offset,
        flags,
        header_size,
        program_entry_size,
        program_count,
        _section_entry_size,
        _section_count,
        _section_name_index,
    ) = fields
    if header_size != header.size or program_entry_size != program_header.size:
        raise ExecutableIdentityError("ELF header or program-header size is unsupported")
    program_end = program_offset + program_count * program_entry_size
    if program_end > len(payload):
        raise ExecutableIdentityError("ELF program-header table is truncated")

    load_segments: list[tuple[int, int, bytes]] = []
    note_segments: list[tuple[int, int]] = []
    raw_program_headers = bytearray()
    for index in range(program_count):
        offset = program_offset + index * program_entry_size
        raw = payload[offset : offset + program_entry_size]
        values = program_header.unpack(raw)
        if elf_class == _ELFCLASS64:
            segment_type, _segment_flags, file_offset, _, _, file_size, _, _ = values
        else:
            segment_type, file_offset, _, _, file_size, _, _segment_flags, _ = values
        if file_offset + file_size > len(payload):
            raise ExecutableIdentityError("ELF segment extends beyond the file")
        raw_program_headers.extend(raw)
        if segment_type == _PT_LOAD:
            load_segments.append((file_offset, file_size, raw))
        elif segment_type == _PT_NOTE:
            note_segments.append((file_offset, file_size))
    if not load_segments:
        raise ExecutableIdentityError("ELF has no loadable segment")

    build_id_ranges = _gnu_build_id_ranges(payload, note_segments, endian)
    digest = hashlib.sha256()
    digest.update(f"bbb:{ELF_LOAD_IMAGE_ALGORITHM}\0".encode("ascii"))
    digest.update(ident)
    digest.update(
        struct.pack(
            ">HHIQQIHHH",
            elf_type,
            machine,
            version,
            entry,
            program_offset,
            flags,
            header_size,
            program_entry_size,
            program_count,
        )
    )
    digest.update(raw_program_headers)
    for file_offset, file_size, raw_header in load_segments:
        segment = bytearray(payload[file_offset : file_offset + file_size])
        for start, size in section_metadata_ranges:
            _zero_file_range(
                segment,
                segment_offset=file_offset,
                start=start,
                size=size,
            )
        for start, size in build_id_ranges:
            _zero_file_range(
                segment,
                segment_offset=file_offset,
                start=start,
                size=size,
            )
        digest.update(raw_header)
        digest.update(segment)
    return {
        "algorithm": ELF_LOAD_IMAGE_ALGORITHM,
        "sha256": digest.hexdigest(),
        "format": "elf",
        "class_bits": 64 if elf_class == _ELFCLASS64 else 32,
        "machine": machine,
        "entry": entry,
        "program_headers": program_count,
        "load_segments": len(load_segments),
    }
