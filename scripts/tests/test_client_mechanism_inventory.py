from __future__ import annotations

import hashlib
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

import client_mechanism_inventory as inventory  # noqa: E402


class ClientMechanismInventoryTests(unittest.TestCase):
    def write_source(self, root: Path, relative: str, source: str) -> None:
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(source, encoding="utf-8")

    def test_method_surface_uses_cst_signatures_lines_and_source_hash(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "fixture/Owner.java"
            source = """\
package fixture;
public class Owner {
  public Owner() {}
  public void decide(int value) {}
  public void decide(String value) {}
}
"""
            self.write_source(root, relative, source)
            builder = inventory.InventoryBuilder(root)
            builder.add_method_surface(
                inventory.MethodSurface("decision", "fixture", "fixture-mechanism", relative)
            )

        self.assertEqual([item["source"]["line"] for item in builder.owners], [3, 4, 5])
        self.assertEqual(len({item["owner_id"] for item in builder.owners}), 3)
        self.assertEqual(
            builder.hashes[relative], hashlib.sha256(source.encode("utf-8")).hexdigest()
        )

    def test_registration_surface_ignores_comments_and_string_literals(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "fixture/Registrations.java"
            self.write_source(
                root,
                relative,
                """\
package fixture;
public class Registrations {
  public static void bootstrap() {
    // register(FAKE, Fake::new);
    String fake = "register(FAKE, Fake::new)";
    register(REAL, Real::new);
  }
}
""",
            )
            builder = inventory.InventoryBuilder(root)
            builder.add_registration_surface(
                "provider", "fixture", "fixture-mechanism", relative, ("register",)
            )

        self.assertEqual(len(builder.owners), 1)
        self.assertIn("REAL", builder.owners[0]["vanilla_owner"])

    def test_synchronized_registry_entries_come_from_list_arguments(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "net/minecraft/resources/RegistryDataLoader.java"
            self.write_source(
                root,
                relative,
                """\
package net.minecraft.resources;
public class RegistryDataLoader {
  public static final List<RegistryData<?>> SYNCHRONIZED_REGISTRIES = List.of(
      new RegistryData<>(Registries.BIOME, Biome.NETWORK_CODEC),
      new RegistryData<>(Registries.CHAT_TYPE, ChatType.DIRECT_CODEC));
}
""",
            )
            builder = inventory.InventoryBuilder(root)
            builder.add_synchronized_registries()

        self.assertEqual(len(builder.owners), 2)
        self.assertIn("Registries.BIOME", builder.owners[0]["vanilla_owner"])
        self.assertIn("Registries.CHAT_TYPE", builder.owners[1]["vanilla_owner"])

    def test_serverbound_scan_finds_constructed_and_singleton_packets(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            relative = "fixture/Actions.java"
            self.write_source(
                root,
                relative,
                """\
package fixture;
public class Actions {
  public void act() {
    connection.send(new ServerboundUseItemPacket());
    connection.send(ServerboundConfigurationAcknowledgedPacket.INSTANCE);
    connection.send(new ClientboundWrongDirectionPacket());
  }
}
""",
            )
            original = inventory.SERVERBOUND_SCAN_PATHS
            inventory.SERVERBOUND_SCAN_PATHS = (relative,)
            try:
                builder = inventory.InventoryBuilder(root)
                builder.add_serverbound_calls()
            finally:
                inventory.SERVERBOUND_SCAN_PATHS = original

        self.assertEqual(
            sorted(item["vanilla_owner"].split(" -> ")[-1] for item in builder.owners),
            ["ServerboundConfigurationAcknowledgedPacket", "ServerboundUseItemPacket"],
        )


if __name__ == "__main__":
    unittest.main()
