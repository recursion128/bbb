import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
SPEC = importlib.util.spec_from_file_location(
    "generate_bootstrap_catalog", SCRIPTS / "generate-bootstrap-catalog.py"
)
assert SPEC is not None and SPEC.loader is not None
GENERATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GENERATOR)


class BootstrapCatalogGeneratorTests(unittest.TestCase):
    def test_vanilla_texture_dimensions_reads_png_ihdr(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            texture = root / "assets/minecraft/textures/entity/test.png"
            texture.parent.mkdir(parents=True)
            texture.write_bytes(
                b"\x89PNG\r\n\x1a\n"
                + b"\x00\x00\x00\x0dIHDR"
                + (37).to_bytes(4, "big")
                + (19).to_bytes(4, "big")
            )

            self.assertEqual(
                GENERATOR.vanilla_texture_dimensions(
                    root, "minecraft:entity/test"
                ),
                (37, 19),
            )

    def test_variant_renderers_use_supplied_texture_dimensions(self) -> None:
        cat = GENERATOR.CatVariantEntry(
            "TABBY",
            "minecraft:tabby",
            "minecraft:entity/cat/adult",
            "minecraft:entity/cat/baby",
        )
        rendered_cat = GENERATOR.render_cat_variants(
            [cat],
            {
                cat.adult_asset_id: (71, 41),
                cat.baby_asset_id: (43, 23),
            },
        )
        self.assertIn("size: [71, 41]", rendered_cat)
        self.assertIn("size: [43, 23]", rendered_cat)

        wolf = GENERATOR.WolfVariantEntry(
            "PALE",
            "minecraft:pale",
            "wolf",
            tuple(f"minecraft:entity/wolf/asset_{index}" for index in range(6)),
        )
        wolf_dimensions = {
            asset_id: (80 + index, 40 + index)
            for index, asset_id in enumerate(wolf.asset_ids)
        }
        rendered_wolf = GENERATOR.render_wolf_variants([wolf], wolf_dimensions)
        for size in wolf_dimensions.values():
            self.assertIn(f"size: [{size[0]}, {size[1]}]", rendered_wolf)

        resources = [
            GENERATOR.BootstrapEntry("TEMPERATE", "minecraft:temperate"),
            GENERATOR.BootstrapEntry("WARM", "minecraft:warm"),
            GENERATOR.BootstrapEntry("COLD", "minecraft:cold"),
        ]
        catalogs = [
            (name, resources)
            for name in (
                "VANILLA_CHICKEN_VARIANT_KEYS",
                "VANILLA_COW_VARIANT_KEYS",
                "VANILLA_PIG_VARIANT_KEYS",
                "VANILLA_FROG_VARIANT_KEYS",
            )
        ]
        farm_catalogs = []
        dimensions = {}
        for family in ("Chicken", "Cow", "Pig"):
            entries = []
            for index, resource in enumerate(resources):
                adult = f"minecraft:entity/{family.lower()}/{resource.constant_name.lower()}"
                baby = f"{adult}_baby"
                dimensions[adult] = (50 + index, 30 + index)
                dimensions[baby] = (25 + index, 15 + index)
                entries.append(
                    GENERATOR.FarmAnimalVariantEntry(
                        resource.constant_name,
                        resource.resource_id,
                        "NORMAL",
                        adult,
                        baby,
                    )
                )
            farm_catalogs.append((family, entries))
        frogs = [
            GENERATOR.FrogVariantEntry(
                resource.constant_name,
                resource.resource_id,
                f"minecraft:entity/frog/{resource.constant_name.lower()}",
            )
            for resource in resources
        ]
        for index, frog in enumerate(frogs):
            dimensions[frog.asset_id] = (91 + index, 61 + index)

        rendered_temperature = GENERATOR.render_temperature_animal_variants(
            catalogs, farm_catalogs, frogs, dimensions
        )
        for frog in frogs:
            size = dimensions[frog.asset_id]
            self.assertIn(f"size: [{size[0]}, {size[1]}]", rendered_temperature)


if __name__ == "__main__":
    unittest.main()
