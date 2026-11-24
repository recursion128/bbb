# 安全边界与生产部署指南

本页记录 BBB 的安全威胁模型、网络隔离要求、MCP 鉴权边界、`bbb-web-host` 代理防护以及生产托管清单。
漏洞披露流程见仓库根 [`SECURITY.md`](../../../SECURITY.md)，完整参数见[命令行与查询参数参考](cli-reference.md)。

## 1. 仅离线（Offline-Only）威胁模型

BBB 客户端与独立服务端（Dedicated Server）明确声明为**仅离线（offline-only）**产品，不集成 Mojang 在线登录、会话验证或
Minecraft 传输层加密：

- **身份假冒风险**：游戏端口（默认 TCP `25565`）不验证玩家对用户名的所有权，也不对数据包加密。
- **部署边界**：生产环境不要将 `bbb-server` 的游戏端口直接暴露到不可信公网；应运行在私有网络、WireGuard/VPN、
  或前置访问控制网关之后，并按需启用 `--white-list true --enforce-whitelist true` 与 `--admin-dir` 持久化名单与封禁表。
- **协议与资源安全预算**：即便在可信网络内，协议线帧长度、zlib 展开大小、NBT 嵌套深度与节点数、Anvil 区域文件
  偏移与扇区长度、数据包与资源包 ZIP 路径及展开字节均受严格上界检查，防止畸形输入触发内存耗尽或路径穿越。

## 2. 服务端网络背压与资源隔离

`bbb-server` 通过以下默认限额隔离慢客户端与畸形流量：

- `--max-connections`（默认 `128`）限制总并发 TCP 任务（含状态探测），`--max-connections-per-ip`（默认 `16`）限制单个远端 IP 的同时连接数；超额直接拒绝并输出 `connection.rejected`。
- `--connection-event-entries` / `--connection-event-bytes`（默认 `64` / `2 MiB`）限制单连接网络→世界入站缓冲，
  `--connection-command-entries` / `--connection-command-bytes`（默认 `64` / `16 MiB`）限制单连接世界→网络出站缓冲，
  避免慢客户端拖垮主世界 `bbb-world` tick 线程。
- 存档写入使用单文件原子替换（`bbb-server-persistence`）与 `session.lock` 排他锁，防止多进程并发打开同一 `--world`。

## 3. MCP 管理端安全边界（本机环回与远程 TLS + OAuth 2.1）

`bbb-server` 的 MCP 管理端（实现位于 `crates/bbb-mcp`、`crates/bbb-server-admin` 与 `crates/bbb-server/src/options.rs`）
遵循「默认仅本机、远程强制加密与标准鉴权」原则：

| 部署模式 | 绑定地址 | 默认权限与硬性前置要求 |
| --- | --- | --- |
| **禁用管理端** | `--no-mcp` | 不启动 HTTP 监听 |
| **本机托管（默认）** | `127.0.0.1:25585`（或 `[::1]`） | 无凭据环回请求授予 `--mcp-loopback-scope`（默认 `manage`，显式传 `none` 可要求本机同样提供令牌） |
| **本机 + 反代网关** | `127.0.0.1:25585` | 由受信外部网关终止 TLS 与身份认证；建议设置 `--mcp-loopback-scope none` 并配合 `--mcp-token-file` |
| **直接远程监听** | 任意非环回 IP | 启动校验**强制要求**同时配置 `--mcp-tls-cert`、`--mcp-tls-key`、`--mcp-oauth-resource` 及配对的 `--mcp-oauth-issuer` / `--mcp-oauth-jwks`，缺任一项直接拒绝启动 |

关键安全不变量：

- **禁止通过命令行参数传递密钥**：`bbb-server`（`--mcp-token-file`）与 `bbb-admin`（`--token-file`）只从文件读取
  Bearer 密钥，避免密钥出现在进程表（`ps`）或 shell 历史中；凭据文件应仅对服务账号可读（如 `chmod 0600`）。
- **最小权限范围（Scope）阶梯**：`read < manage < command`。仅执行巡检与只读查询的调用方应授予 `read`，只有需要执行任意
  `command.execute` 时才授予 `command`。
- **DNS 重绑定（Rebinding）防护**：`--mcp-allowed-origin` 默认为空，默认拒绝所有携带 `Origin` 请求头的浏览器跨站请求；
  仅在明确需要浏览器端直接调用时加入精确来源。
- **TLS 验证**：`bbb-admin` 访问自签或内网 CA 的 HTTPS 管理端时通过 `--ca-file <PEM>` 显式指定受信任 CA。

## 4. `bbb-web-host` 网关安全边界（SSRF / 白名单 / COOP-COEP）

`bbb-web-host` 面向浏览器暴露 HTTP/WebSocket 接口，包含四道明确边界（源码见 `crates/bbb-web-host/src/lib.rs` 与 `crates/bbb-web-host/src/proxy.rs`）：

1. **`/ws` 上游白名单**：仅允许连接到 `{--server} ∪ {--allow-server ...}` 集合中的目标（环回地址在同一端口等价兼容
   `127.0.0.1`、`localhost` 与 `[::1]`），任何表外 `?server=` 请求返回 `403 Forbidden`，防止网关沦为开放 TCP 代理。
2. **`/proxy/skin` 域名白名单**：仅代理 `textures.minecraft.net`、`api.mojang.com`、`sessionserver.mojang.com` 与 `api.minecraftservices.com`，
   限制单次响应最大 `2 MiB`、最多 `3` 次重定向、总超时 `30s`，并同样走 DNS 级 IP 校验（不允许环回或私网）。
3. **`/proxy/pack` SSRF 防护**：
   - 使用自定义 `SsrfSafeDnsResolver` 在 `reqwest` DNS 解析阶段检验每个解析出的目标 IP，直接拒绝环回、私网（RFC 1918 / ULA）、
     链路本地（含云元数据 `169.254.169.254`）等非公网地址，且连接器直接使用本次已校验地址拨号，阻断 DNS 重绑定。
   - 关闭自动重定向并在每一跳（最多 `4` 跳）重新校验协议（scheme）、主机名与解析 IP；单包体积上限 `256 MiB`，总超时 `120s`（连接超时 `15s`、单次读取超时 `30s`；`/proxy/skin` 总超时 `30s`）。
   - 仅限本地开发或自动化测试时显式传 `--allow-pack-loopback` 放行环回地址；局域网自托管资源包可显式传 `--allow-pack-private` 放行 RFC 1918 / ULA 私网地址（仍严格阻断环回、链路本地与云元数据地址，且绝不放宽 `/proxy/skin`），公网生产配置默认两者均关闭。
4. **跨源隔离（COOP/COEP/CORP）与 `/control-ws` 阻断**：
   - 多线程 WASM（`SharedArrayBuffer`）与 WebGPU 要求安全上下文（`https://` 或本机 `localhost`/`127.0.0.1`）及以下响应头：
     `Cross-Origin-Opener-Policy: same-origin`、`Cross-Origin-Embedder-Policy: require-corp`、
     `Cross-Origin-Resource-Policy: same-origin`。
   - 生产部署应将 `bbb-web-host` 绑定在 `127.0.0.1:8080`，关闭 `--mcp-bind` 与 `--control`，前置 Caddy/Nginx 终止 HTTPS
     并在边缘阻断 `/control-ws`（返回 `403`）；现成配置见 [`packaging/README.md`](../../../packaging/README.md)。

## 5. 生产部署最小权限清单

- **专用非特权用户**：使用独立系统用户（如 `bbb`）运行 `bbb-server` 与 `bbb-web-host`，不以 `root` 启动。
- **只读静态目录**：将 `bbb-web-host` 的 `--wasm-dir` 与 `--vanilla-root` 设为对运行用户只读，仅 `--cache-dir`、
  `--world` 与 `--admin-dir` 可写。
- **优雅停机与备份**：systemd（`packaging/bbb-server.service`）与 launchd（`packaging/com.bbb.server.plist`）均通过
  `SIGTERM` 触发停服落盘，确保服务停止超时（如 `60s`）显著大于 `--shutdown-timeout-seconds`（默认 `10s`）。
  导入旧版本存档前先备份整个世界目录，并用官方 26.1 `--forceUpgrade` 升级副本（见[服务端保存与重载](../mechanisms/persistence/server-save-and-reload.md#判断与处理顺序)）。
- **结构化日志审计**：采集 `bbb-server` 的 stderr JSON 日志（`server.ready`、`server.signalled`、`server.stopped`、
  `connection.rejected`、`world.tick_health`）以及 MCP 操作审计记录。

## 相关文档

- [`SECURITY.md`](../../../SECURITY.md)
- [`packaging/README.md`](../../../packaging/README.md)
- [服务端边界与 Agent 管理](../architecture/server-boundaries.md)
- [Web 客户端与网关](web-client.md)
- [服务端 MCP 管理机制](../mechanisms/control/server-mcp-administration.md)
