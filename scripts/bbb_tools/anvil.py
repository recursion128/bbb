"""Read-only Anvil region decoding, for reading a saved world's block truth.

The repository already decodes Anvil saves — `bbb-server-persistence`'s
`RegionFileStore` / `Java26ChunkData` do it exactly, and the round-trip gate
drives them against a real official save. That decoder is a Rust *library*
though: it has no binary, no example and no test that dumps a block at a
coordinate, so reaching it from a Python harness would mean adding an entry
point under `crates/`. This module is the small read-only alternative: enough
of the format to answer "what block is at (x, y, z) in this save", and nothing
else. It never writes, so it can never be a second authority on the save
format — only a reader of one.

Every constant below is derived from the official 26.1 sources, cited by owner:

* `RegionFile` (`SECTOR_BYTES` 4096, `CHUNK_HEADER_SIZE` 5,
  `EXTERNAL_STREAM_FLAG` 128) for the sector table and chunk framing.
* `RegionFileVersion` for the compression ids; `DEFAULT` is `VERSION_DEFLATE`.
* `SerializableChunkData` for the chunk tag names (`sections`, `xPos`, `zPos`,
  `block_states`, `Y`).
* `StateHolder` (`NAME_TAG` / `PROPERTIES_TAG`) for a palette entry's shape.
* `Strategy.getIndex` for the cell order, `(y << 4 | z) << 4 | x`, and
  `Strategy.createForBlockStates` for the palette-size-to-bit-count table.
* `SimpleBitStorage` for the packing: `64 / bits` values per long, no entry
  straddling a long boundary.
"""

from __future__ import annotations

import gzip
import struct
import zlib
from dataclasses import dataclass
from pathlib import Path
from typing import Any

#: `RegionFile.SECTOR_BYTES`. The sector table and the timestamp table are one
#: sector each, so chunk payloads start at sector 2.
SECTOR_BYTES = 4096

#: `RegionFile.EXTERNAL_STREAM_FLAG`: the compression byte's high bit says the
#: payload lives beside the region file in `c.<x>.<z>.mcc` instead.
EXTERNAL_STREAM_FLAG = 128

#: `RegionFileVersion`'s registered ids. LZ4 and the custom slot are declared
#: here so an encounter names itself rather than decoding as garbage; nothing
#: in this repository writes them, because `RegionFileVersion.DEFAULT` is
#: `VERSION_DEFLATE`.
COMPRESSION_GZIP = 1
COMPRESSION_DEFLATE = 2
COMPRESSION_NONE = 3
COMPRESSION_LZ4 = 4
COMPRESSION_CUSTOM = 127

#: `Strategy.createForBlockStates` builds its strategy with `bitsPerAxis` 4, so
#: a section is 16x16x16 cells indexed `(y << 4 | z) << 4 | x`.
SECTION_AXIS = 16
SECTION_CELLS = SECTION_AXIS**3

TAG_END = 0
TAG_BYTE = 1
TAG_SHORT = 2
TAG_INT = 3
TAG_LONG = 4
TAG_FLOAT = 5
TAG_DOUBLE = 6
TAG_BYTE_ARRAY = 7
TAG_STRING = 8
TAG_LIST = 9
TAG_COMPOUND = 10
TAG_INT_ARRAY = 11
TAG_LONG_ARRAY = 12


class AnvilError(RuntimeError):
    """A save did not decode as the 26.1 Anvil format."""


@dataclass(frozen=True, order=True)
class BlockState:
    """One palette entry: `StateHolder`'s `Name` plus its `Properties`."""

    name: str
    properties: tuple[tuple[str, str], ...] = ()

    def describe(self) -> str:
        if not self.properties:
            return self.name
        inner = ",".join(f"{key}={value}" for key, value in self.properties)
        return f"{self.name}[{inner}]"


class _Reader:
    """A cursor over one NBT payload."""

    def __init__(self, data: bytes) -> None:
        self.data = data
        self.offset = 0

    def take(self, count: int) -> bytes:
        end = self.offset + count
        if end > len(self.data):
            raise AnvilError("NBT payload ended early")
        chunk = self.data[self.offset : end]
        self.offset = end
        return chunk

    def unpack(self, format_: str) -> Any:
        size = struct.calcsize(format_)
        return struct.unpack(format_, self.take(size))[0]

    def string(self) -> str:
        length = self.unpack(">H")
        raw = self.take(length)
        # Vanilla writes Java's modified UTF-8. Every name this reader looks at
        # is a resource location or a property name, which is ASCII; latin-1 is
        # the lossless fallback that keeps an exotic byte visible rather than
        # raising here and losing the whole save.
        try:
            return raw.decode("utf-8")
        except UnicodeDecodeError:
            return raw.decode("latin-1")


def _read_payload(reader: _Reader, tag: int) -> Any:
    if tag == TAG_BYTE:
        return reader.unpack(">b")
    if tag == TAG_SHORT:
        return reader.unpack(">h")
    if tag == TAG_INT:
        return reader.unpack(">i")
    if tag == TAG_LONG:
        return reader.unpack(">q")
    if tag == TAG_FLOAT:
        return reader.unpack(">f")
    if tag == TAG_DOUBLE:
        return reader.unpack(">d")
    if tag == TAG_BYTE_ARRAY:
        return _read_byte_array(reader)
    if tag == TAG_STRING:
        return reader.string()
    if tag == TAG_LIST:
        element = reader.unpack(">b")
        count = reader.unpack(">i")
        if count <= 0:
            return []
        return [_read_payload(reader, element) for _ in range(count)]
    if tag == TAG_COMPOUND:
        compound: dict[str, Any] = {}
        while True:
            child = reader.unpack(">b")
            if child == TAG_END:
                return compound
            # The name has to be read into a local first: in `d[k] = v` Python
            # evaluates the value before the subscript, which would take the
            # payload off the cursor ahead of the name it belongs to.
            name = reader.string()
            compound[name] = _read_payload(reader, child)
    if tag == TAG_INT_ARRAY:
        count = reader.unpack(">i")
        return list(struct.unpack(f">{count}i", reader.take(4 * count)))
    if tag == TAG_LONG_ARRAY:
        count = reader.unpack(">i")
        return list(struct.unpack(f">{count}q", reader.take(8 * count)))
    raise AnvilError(f"unknown NBT tag {tag}")


def _read_byte_array(reader: _Reader) -> list[int]:
    count = reader.unpack(">i")
    return list(struct.unpack(f">{count}b", reader.take(count)))


def decode_nbt(payload: bytes) -> dict[str, Any]:
    """Decode one named-root NBT document into plain Python values."""
    reader = _Reader(payload)
    root = reader.unpack(">b")
    if root != TAG_COMPOUND:
        raise AnvilError(f"NBT root is tag {root}, not a compound")
    reader.string()
    value = _read_payload(reader, TAG_COMPOUND)
    if not isinstance(value, dict):  # pragma: no cover - _read_payload contract
        raise AnvilError("NBT root compound did not decode to a mapping")
    return value


def read_compressed_nbt(path: Path) -> dict[str, Any]:
    """Decode a standalone gzipped NBT file such as `level.dat`."""
    return decode_nbt(gzip.decompress(Path(path).read_bytes()))


def region_name(chunk_x: int, chunk_z: int) -> str:
    """The region file holding a chunk: 32x32 chunks per file."""
    return f"r.{chunk_x >> 5}.{chunk_z >> 5}.mca"


def slot_index(chunk_x: int, chunk_z: int) -> int:
    """`RegionFile.getOffsetIndex`: the chunk's entry in the sector table."""
    return (chunk_x & 31) + (chunk_z & 31) * 32


def decompress_chunk(compression: int, payload: bytes) -> bytes:
    if compression == COMPRESSION_GZIP:
        return gzip.decompress(payload)
    if compression == COMPRESSION_DEFLATE:
        return zlib.decompress(payload)
    if compression == COMPRESSION_NONE:
        return payload
    if compression == COMPRESSION_LZ4:
        raise AnvilError(
            "region chunk uses RegionFileVersion.VERSION_LZ4; this reader decodes only "
            "the gzip/deflate/none ids, and RegionFileVersion.DEFAULT is VERSION_DEFLATE"
        )
    if compression == COMPRESSION_CUSTOM:
        raise AnvilError("region chunk uses RegionFileVersion.VERSION_CUSTOM")
    raise AnvilError(f"region chunk declares unknown compression id {compression}")


def chunk_timestamp(region_dir: Path, chunk_x: int, chunk_z: int) -> int | None:
    """When this chunk was last written into its region file, or None if never.

    The region header is two 4 KiB sectors: the offsets, and then the timestamps
    the same buffer is re-read as from position 4096 (RegionFile.java:61-64).
    `RegionFile.write` stamps the slot on every write (RegionFile.java:309) with
    `getTimestamp`, which is epoch seconds (RegionFile.java:155-157).

    That makes it the one thing on disk that says *who wrote last*, which no
    reading of the chunk's own tags can answer: a save one server published and
    another only read back is byte-identical either way.
    """
    path = region_dir / region_name(chunk_x, chunk_z)
    if not path.is_file() or path.stat().st_size == 0:
        return None
    with path.open("rb") as handle:
        header = handle.read(SECTOR_BYTES * 2)
    if len(header) < SECTOR_BYTES * 2:
        raise AnvilError(f"{path} is shorter than its header")
    index = SECTOR_BYTES + slot_index(chunk_x, chunk_z) * 4
    stamp = int.from_bytes(header[index : index + 4], "big")
    return stamp or None


def read_chunk(region_dir: Path, chunk_x: int, chunk_z: int) -> dict[str, Any] | None:
    """The chunk tag at these chunk coordinates, or None when unwritten."""
    path = region_dir / region_name(chunk_x, chunk_z)
    if not path.is_file():
        return None
    # `RegionFile` creates the file when the dimension opens the directory and
    # only writes the 8 KiB header once a chunk actually lands in it, so an
    # empty region file is the normal on-disk shape for "this region holds
    # nothing" -- `entities/` directories are full of them. Vanilla reads such a
    # file with a zero-filled header (RegionFile.java:72-77 warns and carries
    # on), which makes every slot absent; answering None here is that same
    # answer. A file with a partial header is a different thing -- real
    # corruption -- and this reader is evidence tooling, so it fails closed
    # there rather than reporting an empty region.
    if path.stat().st_size == 0:
        return None
    with path.open("rb") as handle:
        table = handle.read(SECTOR_BYTES)
        if len(table) < SECTOR_BYTES:
            raise AnvilError(f"{path} is shorter than its sector table")
        index = slot_index(chunk_x, chunk_z) * 4
        entry = table[index : index + 4]
        sector = int.from_bytes(entry[:3], "big")
        sector_count = entry[3]
        if sector == 0 or sector_count == 0:
            return None
        handle.seek(sector * SECTOR_BYTES)
        header = handle.read(5)
        if len(header) < 5:
            raise AnvilError(f"{path} chunk {chunk_x},{chunk_z} has no chunk header")
        length = int.from_bytes(header[:4], "big")
        compression = header[4]
        if compression & EXTERNAL_STREAM_FLAG:
            external = region_dir / f"c.{chunk_x}.{chunk_z}.mcc"
            if not external.is_file():
                raise AnvilError(
                    f"{path} chunk {chunk_x},{chunk_z} names a missing {external.name}"
                )
            body = external.read_bytes()
            compression &= ~EXTERNAL_STREAM_FLAG
        else:
            body = handle.read(length - 1)
    return decode_nbt(decompress_chunk(compression, body))


def storage_bits(palette_size: int) -> int:
    """On-disc bits per cell for a block-state palette of this size.

    `PalettedContainer.unpack` takes them from
    `Strategy.getConfigurationForPaletteSize`, which is
    `Mth.ceillog2(paletteSize)` widened by
    `Strategy.createForBlockStates`'s table: one entry stores no bits at all,
    two through sixteen entries all store four, and above that the count is the
    ceiling log itself.
    """
    if palette_size <= 0:
        raise AnvilError("a block-state palette cannot be empty")
    if palette_size == 1:
        return 0
    bits = (palette_size - 1).bit_length()
    return max(4, bits)


def unpack_cell(data: list[int], bits: int, index: int) -> int:
    """`SimpleBitStorage.get`: values never straddle a long boundary."""
    if bits <= 0:
        return 0
    per_long = 64 // bits
    cell = index // per_long
    if cell >= len(data):
        raise AnvilError(f"cell {index} is past the {len(data)} longs the section stores")
    shift = (index - cell * per_long) * bits
    # NBT longs are signed; mask back to the unsigned bit pattern before
    # shifting so the top entry of a long is not sign-extended away.
    return ((data[cell] & 0xFFFFFFFFFFFFFFFF) >> shift) & ((1 << bits) - 1)


def cell_index(x: int, y: int, z: int) -> int:
    """`Strategy.getIndex` with `bitsPerAxis` 4."""
    return ((y << 4) | z) << 4 | x


def palette_entry(entry: dict[str, Any]) -> BlockState:
    name = entry.get("Name")
    if not isinstance(name, str):
        raise AnvilError("a palette entry has no Name")
    properties = entry.get("Properties") or {}
    if not isinstance(properties, dict):
        raise AnvilError(f"palette entry {name} has a non-compound Properties")
    return BlockState(
        name=name,
        properties=tuple(sorted((str(key), str(value)) for key, value in properties.items())),
    )


def section_block_state(section: dict[str, Any], x: int, y: int, z: int) -> BlockState | None:
    """The block at a section-local cell, or None when the section is empty."""
    block_states = section.get("block_states")
    if not isinstance(block_states, dict):
        return None
    palette = block_states.get("palette")
    if not isinstance(palette, list) or not palette:
        return None
    bits = storage_bits(len(palette))
    if bits == 0:
        return palette_entry(palette[0])
    data = block_states.get("data")
    if not isinstance(data, list):
        raise AnvilError("a multi-entry block_states palette has no data array")
    index = unpack_cell(data, bits, cell_index(x, y, z))
    if index >= len(palette):
        raise AnvilError(f"palette index {index} is past the {len(palette)}-entry palette")
    return palette_entry(palette[index])


def chunk_block_state(chunk: dict[str, Any], x: int, y: int, z: int) -> BlockState | None:
    """The block at world coordinates inside an already-read chunk tag."""
    sections = chunk.get("sections")
    if not isinstance(sections, list):
        raise AnvilError("chunk tag has no sections list")
    wanted = y >> 4
    for section in sections:
        if not isinstance(section, dict) or int(section.get("Y", 0)) != wanted:
            continue
        return section_block_state(section, x & 15, y & 15, z & 15)
    return None


def chunk_entities(chunk: dict[str, Any]) -> list[dict[str, Any]]:
    """The entity records inside an already-read entity-chunk tag.

    `EntityStorage.loadEntities` reads them out of `"Entities"`
    (EntityStorage.java:32, :74) and treats a chunk with no such list as empty
    (`childrenListOrEmpty`), which is why a missing tag answers `[]` rather than
    raising. The sibling `"Position"` tag (:33, :61) is the chunk's own
    coordinate; callers that want it read it off the tag directly.
    """
    entities = chunk.get("Entities")
    if entities is None:
        return []
    if not isinstance(entities, list):
        raise AnvilError("entity chunk tag has a non-list Entities")
    for entity in entities:
        if not isinstance(entity, dict):
            raise AnvilError("entity chunk tag has a non-compound entity record")
    return entities


class WorldSaveReader:
    """Block and entity truth for one dimension of one save on disk, chunk-cached.

    The two halves live in two sibling region-format directories: `region/`
    (`ChunkMap.java:172`) and `entities/` (`ServerLevel.java:247`). Both are
    read by the same Anvil reader because both are the same container format;
    only the tags inside a chunk differ.
    """

    def __init__(self, world_dir: Path, dimension: str = "overworld") -> None:
        self.world_dir = Path(world_dir)
        dimension_dir = self.world_dir / "dimensions" / "minecraft" / dimension
        self.region_dir = dimension_dir / "region"
        self.entities_dir = dimension_dir / "entities"
        self._chunks: dict[tuple[int, int], dict[str, Any] | None] = {}
        self._entity_chunks: dict[tuple[int, int], dict[str, Any] | None] = {}

    def chunk(self, chunk_x: int, chunk_z: int) -> dict[str, Any] | None:
        key = (chunk_x, chunk_z)
        if key not in self._chunks:
            self._chunks[key] = read_chunk(self.region_dir, chunk_x, chunk_z)
        return self._chunks[key]

    def block_state(self, x: int, y: int, z: int) -> BlockState | None:
        chunk = self.chunk(x >> 4, z >> 4)
        if chunk is None:
            return None
        return chunk_block_state(chunk, x, y, z)

    def entity_chunk(self, chunk_x: int, chunk_z: int) -> dict[str, Any] | None:
        """The raw entity-chunk tag, or `None` when the save holds no such chunk."""
        key = (chunk_x, chunk_z)
        if key not in self._entity_chunks:
            self._entity_chunks[key] = read_chunk(self.entities_dir, chunk_x, chunk_z)
        return self._entity_chunks[key]

    def entities(self, chunk_x: int, chunk_z: int) -> list[dict[str, Any]]:
        """Every entity record the save holds for one chunk column.

        A column the save never wrote and a column written with no entities are
        both `[]`: `EntityStorage.loadEntities` answers `emptyChunk(pos)` for a
        missing region entry (EntityStorage.java:46-57) and an absent
        `"Entities"` list alike.
        """
        chunk = self.entity_chunk(chunk_x, chunk_z)
        if chunk is None:
            return []
        return chunk_entities(chunk)
