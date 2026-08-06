from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPTS = Path(__file__).resolve().parents[1]
REPO_ROOT = SCRIPTS.parent
sys.path.insert(0, str(SCRIPTS))

SPEC = importlib.util.spec_from_file_location(
    "generate_particle_provider_coverage", SCRIPTS / "generate-particle-provider-coverage.py"
)
assert SPEC is not None and SPEC.loader is not None
GENERATOR = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = GENERATOR
SPEC.loader.exec_module(GENERATOR)

from particle_provider_catalog_extractor import (  # noqa: E402
    CODE_SPAWN,
    PROVIDER_SPAWN,
    Fact,
    ParticleFacts,
    ParticleSources,
    extract_particle_facts,
)


BASE_SOURCES = {
    # The interface has to be in the package: the toolkit only keeps `implements`
    # clauses it can resolve against the parsed types.
    "ParticleProvider": """
package net.minecraft.client.particle;

public interface ParticleProvider<T> {
   Particle createParticle(T options, ClientLevel level, double x, double y, double z);
}
""",
    "Particle": """
package net.minecraft.client.particle;

public class Particle {
   protected double xd;
   protected double yd;
   protected double zd;
   protected boolean hasPhysics = true;
   protected float gravity;
   protected int age;
   protected int lifetime;
   protected boolean removed;

   protected Particle(final ClientLevel level, final double x, final double y, final double z) {
      this.lifetime = 4;
   }

   public Particle(
      final ClientLevel level,
      final double x,
      final double y,
      final double z,
      final double xa,
      final double ya,
      final double za
   ) {
      this(level, x, y, z);
      this.xd = xa;
      this.yd = ya;
      this.zd = za;
   }

   public void tick() {
      if (this.age++ >= this.lifetime) {
         this.remove();
      } else {
         this.yd = this.yd - 0.04 * this.gravity;
         this.move(this.xd, this.yd, this.zd);
      }
   }

   public void move(final double xa, final double ya, final double za) {
      this.onGround = false;
   }

   public void remove() {
      this.removed = true;
   }
}
""",
    "NoRenderParticle": """
package net.minecraft.client.particle;

public abstract class NoRenderParticle extends Particle {
   protected NoRenderParticle(final ClientLevel level, final double x, final double y, final double z) {
      super(level, x, y, z);
   }

   @Override
   public ParticleRenderType getGroup() {
      return ParticleRenderType.NO_RENDER;
   }
}
""",
}


class DerivationTests(unittest.TestCase):
    """The four collision opt-outs and the gate folding, on synthetic sources."""

    def facts_for(self, **extra: str) -> dict[str, ParticleFacts]:
        """Derive facts from a throwaway package: the base classes plus `extra`."""
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        package = Path(directory.name) / "net/minecraft/client/particle"
        package.mkdir(parents=True)
        for name, text in {**BASE_SOURCES, **extra}.items():
            (package / f"{name}.java").write_text(text)
        sources = ParticleSources.load(Path(directory.name))
        return {row.name: row for row in extract_particle_facts(sources)}

    def test_a_particle_built_without_velocity_or_gravity_cannot_collide(self) -> None:
        rows = self.facts_for(
            MotionlessParticle="""
package net.minecraft.client.particle;

public class MotionlessParticle extends Particle {
   private MotionlessParticle(final ClientLevel level, final double x, final double y, final double z) {
      super(level, x, y, z);
      this.gravity = 0.0F;
      this.lifetime = 30;
   }

   public static class Provider implements ParticleProvider<SimpleParticleType> {
      public Particle createParticle(final SimpleParticleType options, final ClientLevel level, final double x, final double y, final double z) {
         return new MotionlessParticle(level, x, y, z);
      }
   }
}
"""
        )
        collision = rows["MotionlessParticle.Provider"].collision
        self.assertFalse(collision.present)
        self.assertEqual(collision.reason, "velocity pinned to zero")

    def test_a_particle_given_velocity_collides_through_the_base_tick(self) -> None:
        rows = self.facts_for(
            MovingParticle="""
package net.minecraft.client.particle;

public class MovingParticle extends Particle {
   private MovingParticle(final ClientLevel level, final double x, final double y, final double z, final double xa, final double ya, final double za) {
      super(level, x, y, z, xa, ya, za);
      this.gravity = 0.06F;
   }

   public static class Provider implements ParticleProvider<SimpleParticleType> {
      public Particle createParticle(final SimpleParticleType options, final ClientLevel level, final double x, final double y, final double z, final double xa, final double ya, final double za) {
         return new MovingParticle(level, x, y, z, xa, ya, za);
      }
   }
}
"""
        )
        collision = rows["MovingParticle.Provider"].collision
        self.assertTrue(collision.present)

    def test_a_super_tick_override_still_reaches_the_base_move(self) -> None:
        """The reachability walk resolves `super.tick()` one level up, not onto itself."""
        rows = self.facts_for(
            SpriteParticle="""
package net.minecraft.client.particle;

public class SpriteParticle extends Particle {
   private SpriteParticle(final ClientLevel level, final double x, final double y, final double z, final double xa, final double ya, final double za) {
      super(level, x, y, z, xa, ya, za);
   }

   @Override
   public void tick() {
      super.tick();
      this.setSpriteFromAge();
   }

   public static class Provider implements ParticleProvider<SimpleParticleType> {
      public Particle createParticle(final SimpleParticleType options, final ClientLevel level, final double x, final double y, final double z, final double xa, final double ya, final double za) {
         return new SpriteParticle(level, x, y, z, xa, ya, za);
      }
   }
}
"""
        )
        collision = rows["SpriteParticle.Provider"].collision
        self.assertTrue(collision.present, collision.reason)

    def test_hasPhysics_false_opts_out_even_when_the_tick_moves(self) -> None:
        rows = self.facts_for(
            GhostParticle="""
package net.minecraft.client.particle;

public class GhostParticle extends Particle {
   private GhostParticle(final ClientLevel level, final double x, final double y, final double z, final double xa, final double ya, final double za) {
      super(level, x, y, z, xa, ya, za);
      this.hasPhysics = false;
   }

   public static class Provider implements ParticleProvider<SimpleParticleType> {
      public Particle createParticle(final SimpleParticleType options, final ClientLevel level, final double x, final double y, final double z, final double xa, final double ya, final double za) {
         return new GhostParticle(level, x, y, z, xa, ya, za);
      }
   }
}
"""
        )
        collision = rows["GhostParticle.Provider"].collision
        self.assertFalse(collision.present)
        self.assertEqual(collision.reason, "hasPhysics=false")

    def test_a_construction_constant_can_kill_a_removal_gate(self) -> None:
        """The same particle class gates removal for one provider and not the other."""
        rows = self.facts_for(
            DripFixtureParticle="""
package net.minecraft.client.particle;

public class DripFixtureParticle extends Particle {
   private final Fluid type;

   private DripFixtureParticle(final ClientLevel level, final double x, final double y, final double z, final Fluid type) {
      super(level, x, y, z, 0.0, 0.0, 0.0);
      this.type = type;
      this.gravity = 0.06F;
   }

   @Override
   public void tick() {
      super.tick();
      if (this.type != Fluids.EMPTY) {
         if (this.level.getFluidState(this.pos).is(this.type)) {
            this.remove();
         }
      }
   }

   public static class WaterProvider implements ParticleProvider<SimpleParticleType> {
      public Particle createParticle(final SimpleParticleType options, final ClientLevel level, final double x, final double y, final double z) {
         return new DripFixtureParticle(level, x, y, z, Fluids.WATER);
      }
   }

   public static class NectarProvider implements ParticleProvider<SimpleParticleType> {
      public Particle createParticle(final SimpleParticleType options, final ClientLevel level, final double x, final double y, final double z) {
         return new DripFixtureParticle(level, x, y, z, Fluids.EMPTY);
      }
   }
}
"""
        )
        self.assertTrue(rows["DripFixtureParticle.WaterProvider"].conditional_removal.present)
        nectar = rows["DripFixtureParticle.NectarProvider"].conditional_removal
        self.assertFalse(nectar.present)
        self.assertEqual(nectar.reason, "removal only on lifetime expiry")

    def test_expiry_in_an_else_branch_is_told_apart_from_an_early_removal(self) -> None:
        rows = self.facts_for(
            FadingParticle="""
package net.minecraft.client.particle;

public class FadingParticle extends Particle {
   private float alpha = 1.0F;

   private FadingParticle(final ClientLevel level, final double x, final double y, final double z) {
      super(level, x, y, z);
   }

   @Override
   public void tick() {
      if (this.age++ < this.lifetime && !(this.alpha <= 0.0F)) {
         this.alpha -= 0.015F;
      } else {
         this.remove();
      }
   }

   public static class Provider implements ParticleProvider<SimpleParticleType> {
      public Particle createParticle(final SimpleParticleType options, final ClientLevel level, final double x, final double y, final double z) {
         return new FadingParticle(level, x, y, z);
      }
   }
}
""",
            ExpiringParticle="""
package net.minecraft.client.particle;

public class ExpiringParticle extends Particle {
   private ExpiringParticle(final ClientLevel level, final double x, final double y, final double z) {
      super(level, x, y, z);
   }

   @Override
   public void tick() {
      if (this.age++ < this.lifetime) {
         this.setSpriteFromAge();
      } else {
         this.remove();
      }
   }

   public static class Provider implements ParticleProvider<SimpleParticleType> {
      public Particle createParticle(final SimpleParticleType options, final ClientLevel level, final double x, final double y, final double z) {
         return new ExpiringParticle(level, x, y, z);
      }
   }
}
""",
        )
        self.assertTrue(rows["FadingParticle.Provider"].conditional_removal.present)
        self.assertFalse(rows["ExpiringParticle.Provider"].conditional_removal.present)

    def test_no_render_emitters_need_no_renderer_descriptor(self) -> None:
        rows = self.facts_for(
            SeedParticle="""
package net.minecraft.client.particle;

public class SeedParticle extends NoRenderParticle {
   private SeedParticle(final ClientLevel level, final double x, final double y, final double z) {
      super(level, x, y, z);
   }

   public static class Provider implements ParticleProvider<SimpleParticleType> {
      public Particle createParticle(final SimpleParticleType options, final ClientLevel level, final double x, final double y, final double z) {
         return new SeedParticle(level, x, y, z);
      }
   }
}
"""
        )
        renders = rows["SeedParticle.Provider"].renders
        self.assertFalse(renders.present)
        self.assertIn("NoRenderParticle", renders.reason)

    def test_local_player_and_sound_reads_are_seen_through_tick_helpers(self) -> None:
        rows = self.facts_for(
            ScopingParticle="""
package net.minecraft.client.particle;

public class ScopingParticle extends Particle {
   private ScopingParticle(final ClientLevel level, final double x, final double y, final double z) {
      super(level, x, y, z);
   }

   @Override
   public void tick() {
      super.tick();
      if (this.isCloseToScopingPlayer()) {
         this.level.playLocalSound(this.x, this.y, this.z, SoundEvents.PING, SoundSource.BLOCKS, 1.0F, 1.0F, false);
      }
   }

   private boolean isCloseToScopingPlayer() {
      Minecraft instance = Minecraft.getInstance();
      LocalPlayer player = instance.player;
      return player != null;
   }

   public static class Provider implements ParticleProvider<SimpleParticleType> {
      public Particle createParticle(final SimpleParticleType options, final ClientLevel level, final double x, final double y, final double z) {
         return new ScopingParticle(level, x, y, z);
      }
   }
}
"""
        )
        row = rows["ScopingParticle.Provider"]
        self.assertTrue(row.player_coupled.present)
        self.assertTrue(row.sounds.present)


class LedgerTests(unittest.TestCase):
    def setUp(self) -> None:
        self.ledger = json.loads(GENERATOR.DEFAULT_OUTPUT.read_text())
        self.overrides = GENERATOR.load_overrides(GENERATOR.DEFAULT_OVERRIDES)

    def test_every_row_has_five_valid_cells_and_resolved_evidence(self) -> None:
        evidence_sets = self.ledger["evidence_sets"]
        self.assertEqual(len(self.ledger["entries"]), 113)
        for entry in self.ledger["entries"]:
            self.assertEqual(set(entry["coverage"]), set(GENERATOR.CELL_CATEGORIES))
            for category, cell in entry["coverage"].items():
                self.assertIn(cell["status"], GENERATOR.ALLOWED_STATUSES)
                self.assertIn(cell["evidence_ref"], evidence_sets)
                evidence = evidence_sets[cell["evidence_ref"]]
                if cell["status"] == "complete":
                    self.assertTrue(evidence["implementation"])
                    self.assertTrue(evidence["tests"])
                    self.assertNotIn("gap", cell)
                else:
                    self.assertTrue(evidence["vanilla"])
                    self.assertTrue(cell["gap"])

    def test_not_applicable_cells_carry_the_derivation_that_decided_them(self) -> None:
        reasons = {
            entry["coverage"]["collision"]["gap"]
            for entry in self.ledger["entries"]
            if entry["coverage"]["collision"]["status"] == "not-applicable"
        }
        self.assertTrue(reasons)
        self.assertTrue(
            reasons
            <= {
                "hasPhysics=false",
                "tick never calls move",
                "velocity pinned to zero",
            }
            | {reason for reason in reasons if reason.startswith("collision-free move override")},
            reasons,
        )

    def test_only_particles_that_draw_nothing_go_unbound(self) -> None:
        unbound = [
            entry["name"]
            for entry in self.ledger["entries"]
            if not entry["renderer_descriptor"]
        ]
        self.assertEqual(
            sorted(unbound),
            ["FireworkParticles.Starter", "GustSeedParticle.Provider", "TrackingEmitter"],
        )
        seed = next(
            entry
            for entry in self.ledger["entries"]
            if entry["name"] == "GustSeedParticle.Provider"
        )
        self.assertEqual(seed["coverage"]["renderer binding"]["status"], "not-applicable")

    def test_new_vanilla_facts_require_an_explicit_audit(self) -> None:
        overrides = dict(self.overrides, catalog_sha256="deadbeef")
        with self.assertRaisesRegex(ValueError, "derived particle provider facts changed"):
            GENERATOR.render_ledger(
                [], GENERATOR.RendererBinding("", frozenset(), {}), overrides, REPO_ROOT
            )

    def test_a_live_behaviour_the_renderer_does_not_bind_must_be_classified(self) -> None:
        row = ParticleFacts(
            name="FixtureParticle.Provider",
            particle="FixtureParticle",
            spawn=PROVIDER_SPAWN,
            renders=Fact(True, "draws through a particle render group"),
            collision=Fact(True, "colliding move reachable from tick"),
            player_coupled=Fact(False, "tick reads no local player or camera"),
            sounds=Fact(False, "tick raises no playLocalSound"),
            conditional_removal=Fact(False, "removal only on lifetime expiry"),
        )
        if not GENERATOR.DEFAULT_SOURCE_ROOT.is_dir():
            self.skipTest("local vanilla 26.1 sources are unavailable")
        overrides = dict(
            self.overrides,
            catalog_sha256=GENERATOR.catalog_sha256([row]),
            row_overrides={},
        )
        with self.assertRaisesRegex(ValueError, "classify the cell in the overrides file"):
            GENERATOR.render_ledger(
                [row],
                GENERATOR.RendererBinding("", frozenset(), {}),
                overrides,
                GENERATOR.DEFAULT_SOURCE_ROOT,
            )

    def test_a_renderer_physics_flag_may_not_contradict_vanilla(self) -> None:
        row = ParticleFacts(
            name="FixtureParticle.Provider",
            particle="FixtureParticle",
            spawn=PROVIDER_SPAWN,
            renders=Fact(True, "draws through a particle render group"),
            collision=Fact(False, "hasPhysics=false"),
            player_coupled=Fact(False, "tick reads no local player or camera"),
            sounds=Fact(False, "tick raises no playLocalSound"),
            conditional_removal=Fact(False, "removal only on lifetime expiry"),
        )
        binding = GENERATOR.RendererBinding(
            f'"{row.name}"', frozenset({row.name}), {row.name: True}
        )
        with self.assertRaisesRegex(ValueError, "disagrees with Vanilla"):
            GENERATOR.cross_check_physics([row], binding)

    def test_derived_cells_may_not_be_overridden_by_hand(self) -> None:
        overrides = dict(
            self.overrides,
            row_overrides={
                "AshParticle.Provider": {
                    "collision": {
                        "status": "deferred",
                        "evidence_ref": "particle_collision",
                        "gap": "docs/unsupported-features.md#fixture",
                    }
                },
                "FireworkParticles.Starter": self.overrides["row_overrides"][
                    "FireworkParticles.Starter"
                ],
            },
        )
        sources = GENERATOR.DEFAULT_SOURCE_ROOT
        if not sources.is_dir():
            self.skipTest("local vanilla 26.1 sources are unavailable")
        facts = extract_particle_facts(ParticleSources.load(sources))
        binding = GENERATOR.parse_renderer_binding(GENERATOR.DEFAULT_DESCRIPTORS)
        with self.assertRaisesRegex(ValueError, "derived cell must not be overridden"):
            GENERATOR.render_ledger(facts, binding, overrides, sources)

    def test_override_loader_rejects_duplicate_json_keys(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "overrides.json"
            path.write_text('{"schema_version":1,"uncertainties":[],"uncertainties":[]}')
            with self.assertRaisesRegex(ValueError, "duplicate JSON key 'uncertainties'"):
                GENERATOR.load_overrides(path)

    def test_checked_in_output_is_current_when_vanilla_sources_are_available(self) -> None:
        if not GENERATOR.DEFAULT_SOURCE_ROOT.is_dir():
            self.skipTest("local vanilla 26.1 sources are unavailable")
        facts = extract_particle_facts(ParticleSources.load(GENERATOR.DEFAULT_SOURCE_ROOT))
        binding = GENERATOR.parse_renderer_binding(GENERATOR.DEFAULT_DESCRIPTORS)
        GENERATOR.cross_check_physics(facts, binding)
        rendered = GENERATOR.render_ledger(
            facts, binding, self.overrides, GENERATOR.DEFAULT_SOURCE_ROOT
        )
        self.assertEqual(rendered, GENERATOR.DEFAULT_OUTPUT.read_text())


if __name__ == "__main__":
    unittest.main()
