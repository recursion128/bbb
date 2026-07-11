from __future__ import annotations

import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from jukebox_song_catalog_extractor import (  # noqa: E402
    JukeboxSongCatalogExtractionError,
    jukebox_song_entries,
)
from vanilla_java_source import JavaSource, parse_source  # noqa: E402


def sound_events_source() -> JavaSource:
    return parse_source(
        "net/minecraft/sounds/SoundEvents.java",
        b'''package net.minecraft.sounds;
class SoundEvents {
  static final Holder MUSIC_DISC_FIRST = registerForHolder("music_disc.first");
  static final Holder MUSIC_DISC_SECOND = registerForHolder("music_disc.second");
}''',
    )


def jukebox_source(second_sound: bytes = b"SoundEvents.MUSIC_DISC_SECOND") -> JavaSource:
    return parse_source(
        "net/minecraft/world/item/JukeboxSongs.java",
        b'''package net.minecraft.world.item;
interface JukeboxSongs {
  ResourceKey<JukeboxSong> FIRST = create("first");
  ResourceKey<JukeboxSong> SECOND = create("second");
  static void bootstrap(BootstrapContext<JukeboxSong> context) {
    register(context, FIRST, SoundEvents.MUSIC_DISC_FIRST, 1, 1);
    register(context, SECOND, '''
        + second_sound
        + b''', 2, 2);
  }
}''',
    )


class JukeboxSongCatalogExtractorTests(unittest.TestCase):
    def test_bootstrap_order_joins_song_and_sound_event_resources(self) -> None:
        entries = jukebox_song_entries(jukebox_source(), sound_events_source(), 2, 2)
        self.assertEqual(
            [
                (entry.constant_name, entry.resource_id, entry.sound_event_id)
                for entry in entries
            ],
            [
                ("FIRST", "minecraft:first", "minecraft:music_disc.first"),
                ("SECOND", "minecraft:second", "minecraft:music_disc.second"),
            ],
        )

    def test_unknown_sound_constant_fails_with_source_span(self) -> None:
        with self.assertRaisesRegex(
            JukeboxSongCatalogExtractionError,
            r"JukeboxSongs\.java:\d+:\d+: jukebox sound constant does not exist",
        ):
            jukebox_song_entries(
                jukebox_source(b"SoundEvents.MISSING"), sound_events_source(), 2, 2
            )


if __name__ == "__main__":
    unittest.main()
