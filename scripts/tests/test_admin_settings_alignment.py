"""The bbb management surface must name real vanilla management methods.

Two declarations are guarded here, both against
`net/minecraft/server/jsonrpc/IncomingRpcMethods.java` read through the shared
Tree-sitter Java frontend:

* `crates/bbb-server-core/src/admin/settings.rs` declares the subset of
  vanilla's `serversettings/<key>` methods this server honors. The subset is a
  product decision; the *keys* are vanilla facts.
* `crates/bbb-server-admin/src/tools.rs` declares, per MCP tool, which vanilla
  method it projects. A tool may legitimately project none (bbb owns a few
  surfaces vanilla has no method for), but it must never claim one vanilla does
  not register, and a vanilla method must not silently go unprojected. That
  derivation lives in `scripts/server_management_surface.py`, which the
  management evidence generator reads too; this file only asserts against it.
"""

from __future__ import annotations

import re
import sys
import unittest
from pathlib import Path

import pytest

pytestmark = pytest.mark.vanilla


REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "scripts"))

import server_management_surface as surface  # noqa: E402

SETTINGS_SOURCE = REPO_ROOT / "crates/bbb-server-core/src/admin/settings.rs"
KEYS_BLOCK = re.compile(
    r"pub const ADMIN_SETTING_KEYS: &\[&str\] = &\[(?P<body>.*?)\];", re.DOTALL
)

# Vanilla settings this server deliberately does not own, each with the reason
# no value would back it. Move a key to ADMIN_SETTING_KEYS when implemented.
NOT_OWNED = {
    "autosave": "the world is saved on shutdown, not on a timer",
    "force_game_mode": "every player gets the one fixture game mode",
    "player_idle_timeout": "idle players are not kicked",
    "status_heartbeat_interval": "no status heartbeat is emitted",
}


def vanilla_setting_keys() -> set[str]:
    keys = set()
    for method in surface.vanilla_rpc_methods():
        if not method.startswith("serversettings/"):
            continue
        keys.add(method.removeprefix("serversettings/").removesuffix("/set"))
    return keys


def bbb_tool_methods() -> dict[str, str | None]:
    return {
        str(tool["name"]): tool["vanilla_method"] for tool in surface.bbb_tools()
    }


def bbb_setting_keys() -> list[str]:
    match = KEYS_BLOCK.search(SETTINGS_SOURCE.read_text(encoding="utf-8"))
    assert match is not None, "ADMIN_SETTING_KEYS must stay a flat literal list"
    return re.findall(r'"([^"]+)"', match.group("body"))


class AdminSettingsAlignmentTests(unittest.TestCase):
    def test_vanilla_registers_the_settings_surface_this_guard_reads(self) -> None:
        keys = vanilla_setting_keys()
        self.assertIn("difficulty", keys)
        self.assertIn("max_players", keys)
        self.assertGreater(len(keys), 15)

    def test_every_owned_key_is_a_vanilla_serversettings_key(self) -> None:
        vanilla = vanilla_setting_keys()
        owned = bbb_setting_keys()
        self.assertEqual(sorted(owned), sorted(set(owned)), "duplicate owned key")
        for key in owned:
            self.assertIn(key, vanilla, f"{key} is not a vanilla serversettings key")

    def test_every_vanilla_key_is_owned_or_declared_not_owned(self) -> None:
        vanilla = vanilla_setting_keys()
        owned = set(bbb_setting_keys())
        self.assertEqual(
            owned & NOT_OWNED.keys(),
            set(),
            "a key cannot be both owned and declared not owned",
        )
        self.assertEqual(
            vanilla - owned - NOT_OWNED.keys(),
            set(),
            "classify every vanilla serversettings key as owned or not owned",
        )
        self.assertEqual(
            NOT_OWNED.keys() - vanilla,
            set(),
            "a not-owned key must still be a vanilla serversettings key",
        )


class ToolProjectionAlignmentTests(unittest.TestCase):
    def test_every_declared_vanilla_method_is_registered_by_vanilla(self) -> None:
        registered = surface.vanilla_rpc_methods()
        tools = bbb_tool_methods()
        self.assertGreater(len(tools), 30, "the tool catalog failed to parse")
        claimed = [method for method in tools.values() if method is not None]
        self.assertEqual(sorted(claimed), sorted(set(claimed)), "duplicate projection")
        for name, method in tools.items():
            if method is None:
                continue
            self.assertIn(
                method, registered, f"{name} claims unregistered method {method}"
            )

    def test_every_vanilla_method_is_projected_or_declared_unprojected(self) -> None:
        projected = {
            method for method in bbb_tool_methods().values() if method is not None
        }
        unprojected = {
            method
            for method in surface.vanilla_rpc_methods()
            if not method.startswith("serversettings/")
        } - projected
        self.assertEqual(
            unprojected,
            set(surface.NOT_PROJECTED),
            "classify every vanilla management method as projected or not",
        )

    def test_the_shared_projection_resolves_without_a_gap(self) -> None:
        # The same derivation the management evidence renders from. It refuses
        # every incompleteness itself, so reaching a result is the assertion.
        resolved = surface.projection()
        self.assertEqual(
            sorted(str(tool["name"]) for tool in resolved["tools"]),
            sorted(surface.admin_actions()),
        )


if __name__ == "__main__":
    unittest.main()
