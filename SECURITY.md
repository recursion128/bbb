# Security Policy

## Supported Versions

`bbb` targets Minecraft Java Edition **26.1** on the `main` branch. Security fixes are applied fix-forward on `main`.

## Security Model Overview

Before reporting an issue or deploying `bbb`, note the declared product boundaries (detailed in [Security and Deployment](docs/wiki/development/security-and-deployment.md)):

- **Offline-only Minecraft protocol**: Both `bbb-client` and `bbb-server` operate strictly in offline mode without Mojang session authentication or Minecraft wire encryption. Expose `bbb-server` only on trusted LANs, VPNs, or networks protected by external access controls and `--white-list` / `--enforce-whitelist`.
- **MCP management endpoint (`bbb-server` / `bbb-mcp`)**: Binds to loopback (`127.0.0.1:25585`) by default and refuses non-loopback binds unless TLS (`--mcp-tls-cert`, `--mcp-tls-key`) and OAuth 2.1 (`--mcp-oauth-resource`, `--mcp-oauth-issuer`, `--mcp-oauth-jwks`) are all configured. Pass bearer secrets only via `--mcp-token-file` / `--token-file` (never via CLI arguments). Browser `Origin` headers are rejected by default (`--mcp-allowed-origin` is empty) to mitigate DNS rebinding.
- **Web gateway (`bbb-web-host`)**: Restricts `/ws` upstream TCP connections to `{--server} ∪ {--allow-server ...}`, restricts `/proxy/skin` to Mojang hosts, enforces DNS-level SSRF IP checks and size/redirect bounds on `/proxy/pack`, and emits `COOP`/`COEP`/`CORP` cross-origin isolation headers. In public deployments, terminate HTTPS at a reverse proxy and block `/control-ws` (see [`packaging/README.md`](packaging/README.md)).
- **Untrusted wire, NBT, archive, and pack inputs**: Protocol framing, compression, NBT depth/size/entry counts, Anvil region storage, and resource/data pack paths enforce explicit memory, depth, and path-traversal budgets.

## Reporting a Vulnerability

Please **do not** open a public GitHub issue for security vulnerabilities.

Instead, report vulnerabilities privately through GitHub's **Security Advisories** ("Report a vulnerability") on this repository, or contact the repository maintainers directly with:

1. Affected component (`bbb-server`, `bbb-client`, `bbb-web-host`, `bbb-mcp`, `bbb-protocol`, etc.) and commit SHA.
2. Reproduction steps or a minimal proof of concept.
3. Impact assessment (crash/DoS, SSRF, auth bypass, path traversal, memory exhaustion, etc.).

Maintainers aim to acknowledge reports within 72 hours and coordinate a fix before public disclosure.
