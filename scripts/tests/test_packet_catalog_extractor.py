from __future__ import annotations

import sys
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

from packet_catalog_extractor import (  # noqa: E402
    PacketCatalogExtractionError,
    packet_resource_ids,
    registered_packets,
)
from vanilla_java_source import parse_source  # noqa: E402


PACKET_TYPES = b"""
package fixture;
public class FixturePacketTypes {
  public static final PacketType<FirstPacket> FIRST = createServerbound("first");
  public static final PacketType<BundlePacket> CLIENTBOUND_BUNDLE =
      createClientbound("bundle");
  public static final PacketType<DelimiterPacket> CLIENTBOUND_BUNDLE_DELIMITER =
      createClientbound("bundle_delimiter");
  public static final PacketType<SecondPacket> CLIENTBOUND_SECOND =
      createClientbound("second");
  private static String fake = "createClientbound(\\\"fake\\\")";
}
"""


PROTOCOLS = b"""
package fixture;
public class FixtureProtocols {
  public static final Protocol SERVERBOUND_TEMPLATE = make(
      builder -> builder
          // .addPacket(FixturePacketTypes.FAKE, FakePacket.STREAM_CODEC)
          .addPacket(FixturePacketTypes.FIRST, FirstPacket.STREAM_CODEC));
  public static final Protocol CLIENTBOUND_TEMPLATE = make(
      builder -> builder.withBundlePacket(
              FixturePacketTypes.CLIENTBOUND_BUNDLE,
              BundlePacket::new,
              new DelimiterPacket())
          .addPacket(
              FixturePacketTypes.CLIENTBOUND_SECOND,
              Outer.SecondPacket.CONTEXT_FREE_STREAM_CODEC,
              CODEC_MODIFIER));
}
"""


class PacketCatalogExtractorTests(unittest.TestCase):
    def test_cst_extracts_resources_registration_order_bundle_and_nested_codec_owner(self) -> None:
        resources = packet_resource_ids(
            [parse_source("fixture/FixturePacketTypes.java", PACKET_TYPES)]
        )
        source = parse_source("fixture/FixtureProtocols.java", PROTOCOLS)
        self.assertEqual(resources["FIRST"], "minecraft:first")
        self.assertNotIn("FAKE", resources)
        self.assertEqual(
            registered_packets(source, "serverbound", resources)[0].class_name,
            "FirstPacket",
        )
        clientbound = registered_packets(source, "clientbound", resources)
        self.assertEqual(
            [(item.constant, item.resource_id, item.class_name) for item in clientbound],
            [
                (
                    "CLIENTBOUND_BUNDLE_DELIMITER",
                    "minecraft:bundle_delimiter",
                    "DelimiterPacket",
                ),
                (
                    "CLIENTBOUND_SECOND",
                    "minecraft:second",
                    "Outer.SecondPacket",
                ),
            ],
        )

    def test_unsupported_packet_type_factory_fails_with_source_location(self) -> None:
        source = parse_source(
            "fixture/BadPacketTypes.java",
            PACKET_TYPES.replace(b'createServerbound("first")', b'guess("first")'),
        )
        with self.assertRaisesRegex(
            PacketCatalogExtractionError,
            r"fixture/BadPacketTypes\.java:\d+:\d+: .*unsupported factory guess",
        ):
            packet_resource_ids([source])

    def test_unresolved_registration_never_silently_skips(self) -> None:
        resources = packet_resource_ids(
            [parse_source("fixture/FixturePacketTypes.java", PACKET_TYPES)]
        )
        source = parse_source(
            "fixture/BadProtocols.java",
            PROTOCOLS.replace(b"FixturePacketTypes.FIRST", b"FixturePacketTypes.MISSING"),
        )
        with self.assertRaisesRegex(
            PacketCatalogExtractionError, "missing PacketTypes resource id for MISSING"
        ):
            registered_packets(source, "serverbound", resources)


if __name__ == "__main__":
    unittest.main()
