"""Derive the bbb management surface and bind it to vanilla 26.1.

bbb replaces vanilla's WebSocket JSON-RPC management server with one MCP
Streamable HTTP endpoint. That is a projection, not a reimplementation, so the
facts that decide whether the projection is complete are all derivable:

* the vanilla methods come from ``IncomingRpcMethods.register`` calls;
* the bbb tools come from the static table in ``bbb-server-admin/src/tools.rs``,
  which declares per tool which vanilla method it projects;
* the request surface those tools decode to comes from ``AdminRequest::action``
  in ``bbb-server-core/src/admin/protocol.rs``;
* the pinned MCP revision and the resource catalog come from the adapter.

This module owns that derivation once. The alignment test and the evidence
generator both read it, so neither restates a rule the other enforces.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
if str(REPO_ROOT / "scripts") not in sys.path:
    sys.path.insert(0, str(REPO_ROOT / "scripts"))

from bbb_tools.vanilla.workspace import VanillaWorkspace  # noqa: E402
from vanilla_java_source import parse_source  # noqa: E402

INCOMING_RPC_METHODS = Path("net/minecraft/server/jsonrpc/IncomingRpcMethods.java")

TOOLS_SOURCE = Path("crates/bbb-server-admin/src/tools.rs")
RESOURCES_SOURCE = Path("crates/bbb-server-admin/src/resources.rs")
SERVER_SOURCE = Path("crates/bbb-mcp/src/server.rs")
PROTOCOL_SOURCE = Path("crates/bbb-server-core/src/admin/protocol.rs")

TOOL_ENTRY = re.compile(
    r'name: "(?P<name>[^"]+)",\s*\n'
    r"\s*title: .*?\n"
    r"\s*summary: .*?\n"
    r'\s*vanilla_method: (?:Some\("(?P<method>[^"]+)"\)|None),\s*\n'
    r"\s*destructive: (?P<destructive>true|false),",
    re.DOTALL,
)
RESOURCE_URI = re.compile(r'uri: "(?P<uri>bbb://server/[^"]+)"')
PROTOCOL_VERSION = re.compile(r'pub const PROTOCOL_VERSION: &str = "(?P<version>[^"]+)"')
ACTION_ARM = re.compile(r'=> "(?P<action>[a-z_]+(?:\.[a-z_]+)*)"')
ACTION_BLOCK = re.compile(
    r"pub fn action\(&self\) -> &'static str \{(?P<body>.*?)\n    \}", re.DOTALL
)

# Vanilla management methods bbb deliberately does not project as a tool, each
# with the reason. `serversettings/*` is excluded separately: the settings tool
# and `ADMIN_SETTING_KEYS` own that whole family.
NOT_PROJECTED = {
    "rpc.discover": "MCP discovery is initialize plus tools/list, not an RPC method",
}

SETTINGS_PREFIX = "serversettings/"


class ManagementSurfaceError(RuntimeError):
    """The management projection no longer matches what it projects."""


def vanilla_sources() -> Path:
    return VanillaWorkspace.resolve("26.1").sources


def vanilla_rpc_methods(sources: Path | None = None) -> set[str]:
    """Every method vanilla's management server registers."""
    sources = sources or vanilla_sources()
    source = parse_source(
        INCOMING_RPC_METHODS.as_posix(),
        (sources / INCOMING_RPC_METHODS).read_bytes(),
    )
    methods = set()
    for call in source.calls:
        if call.name != "register" or len(call.arguments) != 2:
            continue
        methods.add(call.arguments[1].canonical.strip('"'))
    return methods


def bbb_tools(repo_root: Path | None = None) -> list[dict[str, object]]:
    """Every declared MCP tool, in catalog order."""
    repo_root = repo_root or REPO_ROOT
    body = (repo_root / TOOLS_SOURCE).read_text(encoding="utf-8")
    tools = [
        {
            "name": match.group("name"),
            "vanilla_method": match.group("method"),
            "destructive": match.group("destructive") == "true",
        }
        for match in TOOL_ENTRY.finditer(body)
    ]
    if not tools:
        raise ManagementSurfaceError(f"{TOOLS_SOURCE}: no tool entries were parsed")
    names = [tool["name"] for tool in tools]
    if len(set(names)) != len(names):
        raise ManagementSurfaceError(f"{TOOLS_SOURCE}: a tool name is declared twice")
    return tools


def bbb_resource_uris(repo_root: Path | None = None) -> list[str]:
    repo_root = repo_root or REPO_ROOT
    body = (repo_root / RESOURCES_SOURCE).read_text(encoding="utf-8")
    uris = [match.group("uri") for match in RESOURCE_URI.finditer(body)]
    if not uris:
        raise ManagementSurfaceError(f"{RESOURCES_SOURCE}: no resources were parsed")
    return uris


def pinned_protocol_version(repo_root: Path | None = None) -> str:
    repo_root = repo_root or REPO_ROOT
    body = (repo_root / SERVER_SOURCE).read_text(encoding="utf-8")
    match = PROTOCOL_VERSION.search(body)
    if match is None:
        raise ManagementSurfaceError(f"{SERVER_SOURCE}: no pinned MCP revision")
    return match.group("version")


def admin_actions(repo_root: Path | None = None) -> list[str]:
    """Every audit action the world's request surface can produce."""
    repo_root = repo_root or REPO_ROOT
    body = (repo_root / PROTOCOL_SOURCE).read_text(encoding="utf-8")
    block = ACTION_BLOCK.search(body)
    if block is None:
        raise ManagementSurfaceError(f"{PROTOCOL_SOURCE}: no AdminRequest::action")
    actions = [match.group("action") for match in ACTION_ARM.finditer(block.group("body"))]
    if not actions:
        raise ManagementSurfaceError(f"{PROTOCOL_SOURCE}: no audit actions were parsed")
    return actions


def projection(
    *, repo_root: Path | None = None, sources: Path | None = None
) -> dict[str, object]:
    """The resolved projection, refusing every way it could be incomplete.

    A tool that claims a method vanilla does not register, a vanilla method that
    silently goes unprojected, and a tool that is not exactly one request are
    each a surface that drifted from what it projects.
    """
    tools = bbb_tools(repo_root)
    methods = vanilla_rpc_methods(sources)
    actions = admin_actions(repo_root)

    claimed = {
        str(tool["vanilla_method"]) for tool in tools if tool["vanilla_method"] is not None
    }
    invented = sorted(claimed - methods)
    if invented:
        raise ManagementSurfaceError(
            f"{TOOLS_SOURCE}: vanilla registers no such method: {invented}"
        )

    unprojected = sorted(
        method
        for method in methods - claimed
        if method not in NOT_PROJECTED and not method.startswith(SETTINGS_PREFIX)
    )
    if unprojected:
        raise ManagementSurfaceError(
            f"vanilla management methods reach no bbb tool: {unprojected}"
        )

    tool_names = {str(tool["name"]) for tool in tools}
    action_names = set(actions)
    if tool_names != action_names:
        missing = sorted(action_names - tool_names)
        extra = sorted(tool_names - action_names)
        raise ManagementSurfaceError(
            "the tool catalog and the request surface disagree: "
            f"unreachable requests {missing}, tools without a request {extra}"
        )

    return {
        "protocol_version": pinned_protocol_version(repo_root),
        "tools": tools,
        "resources": bbb_resource_uris(repo_root),
        "vanilla_methods": sorted(methods),
        "projected_methods": sorted(claimed),
        "settings_family": sorted(
            method for method in methods if method.startswith(SETTINGS_PREFIX)
        ),
        "not_projected": dict(sorted(NOT_PROJECTED.items())),
    }
