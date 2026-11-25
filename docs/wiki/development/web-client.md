# Web 客户端与网关

`bbb-client`（`wasm32-unknown-unknown` `cdylib`）与宿主机轻量网关 `bbb-web-host`
共同提供浏览器端 Minecraft Java 26.1 多人外部客户端。浏览器侧运行完整 Rust 客户端模拟、
WebGPU（WGSL）渲染管线、`SharedArrayBuffer` + `wasm-bindgen-rayon` Web Worker 并发网格
烘培与 Web Audio 3D 音效；`bbb-web-host` 在同源 HTTP 下提供跨源隔离响应头、静态与按需资源、
WebSocket ↔ TCP 二进制协议桥接、皮肤与服务端资源包 CORS 安全代理，以及反向 MCP / Control 隧道。

与桌面端一致，Web 端不提供游戏内配置界面；启动配置由 `bbb-web-host` 命令行参数与浏览器 URL
查询参数在启动时确定，打开页面直接进入连接与游玩流程。

## 架构边界与职责

| 组件 | 路径 | 唯一职责 |
| --- | --- | --- |
| WASM 客户端入口 | `crates/bbb-client/src/web.rs` | 浏览器启动序列、Cache Storage 核心资源缓存、WebGPU 异步初始化、Pointer Lock/全屏/剪贴板、`localStorage` 持久化、F2 异步截图与 `/control-ws` 反向控制客户端 |
| 按需语言包与 RTL | `crates/bbb-client/src/web/locales.rs` | 解析 `asset-index.json` 并按需拉取、SHA-1/长度核验与挂载全部约 120 种非内置 Vanilla 语言包（含阿拉伯语、希伯来语等 RTL 语言）至内存 `virtual_store`（响应复用浏览器不可变 HTTP 缓存） |
| 浏览器 CJK 输入法与快捷键透传 | `crates/bbb-client/src/web/ime.rs` | 隐藏 DOM 输入框（`#bbb-ime-input`）随 `set_ime_allowed` 动态挂载输入法合成候选（文本编辑界面激活、3D 游玩态禁用），并对 `F12`、`Ctrl+Shift+I`、`Cmd+Option+I` 浏览器开发者工具快捷键原样放行 |
| 本地资源包挂载 | `crates/bbb-client/src/web/packs.rs` | 支持页面拖放、`#bbb-pack-picker` 文件选择器与 `?pack=<url>` 参数挂载 `.zip` 资源包并触发原子重载 |
| Web Speech 旁白器 | `crates/bbb-client/src/web/speech.rs` | 基于浏览器 `window.speechSynthesis` 实现 `SystemNarrator`（响应 `Ctrl+B` 切换及系统/聊天旁白队列） |
| 离主线程网络 Worker | `crates/bbb-client/src/web/net_worker.rs` | 在独立共享内存 Web Worker 上运行 `bbb-client-net`（二进制 WebSocket、zlib 压缩解压与协议编解码），经无锁有界队列与 `Window` 模拟线程通信 |
| 调试性能报告下载 | `crates/bbb-client/src/debug_profiler/report.rs` | 浏览器端 `F3+L` 性能剖析完成后在内存生成 `.zip` 报告并触发 Blob 下载 |
| 共享事件循环辅助 | `crates/bbb-client/src/event_loop_shared.rs` | 桌面与 Web 共用的窗口事件映射、平台副作用接口 `PlatformEffectTarget` 与截图完成分发 |
| 传输层抽象 | `crates/bbb-client-net/src/transport.rs` | 原生 `TcpStream` 与浏览器二进制 `WebSocket` 共用 `ClientStream` trait，复用同一套 Login/Configuration/Play 状态机 |
| 虚拟内存资源存储 | `crates/bbb-client-assets/src/virtual_store.rs` | WASM 环境下承载 `core-26.1.zip`、按需语言对象与本地/服务端资源包字节流，供 `PackRoots::from_web_bundle` 与 `KnownPacksManager` 统一读取 |
| Web Audio 后端 | `crates/bbb-client-audio/src/webaudio.rs` | 基于 `AudioContext`、`PannerNode`（HRTF）与 `GainNode` 的 3D 音频后端；`.ogg` 资源按需拉取、SHA-1 校验并缓存至浏览器 Cache Storage，优先通过原生 `AudioContext::decodeAudioData` 离主线程异步解码（特殊头由 `symphonia` 解码） |
| 纯 Rust TTF 光栅 | `crates/bbb-client-render/src/item_model/font/ttf.rs` | 桌面与 `wasm32` 目标统一使用纯 Rust `ab_glyph` 光栅化 TrueType 字体（无原生 FreeType 动态库依赖），保持一致的字体提供器合同 |
| 宿主机网关 | `crates/bbb-web-host` | COOP/COEP/CORP 隔离头、`core-26.1.zip` 构建与磁盘缓存、`/ws` TCP 桥接（含目标白名单）、`/proxy/skin` 与 `/proxy/pack` SSRF 防护代理、`/control-ws` 反向暴露 `--mcp-bind` 与 `--control`（零 `bbb-client`/`wgpu`/`winit` 依赖） |

## 构建与运行

仓库固定 `nightly-2026-08-10` 工具链（含 `rust-src` 与 `wasm32-unknown-unknown` 编译目标）。
WASM 构建使用共享构建缓存根目录（`<target-root>`，依次取 `BBB_TARGET_ROOT`、Linux/macOS 的 `/tmp` 或 Windows 的 `%TEMP%`）下的独立目录 `bbb-target-wasm`，不会污染主树 `bbb-target-main` 的编译指纹；打包产物与 `core-26.1.zip` 缓存默认同落在该根目录下的 `bbb-web-dist` 与 `bbb-web-cache`（`cargo build-web`、独立启动的 `bbb-web-host` 与 `cargo dev e2e web` 共享同一 `<target-root>` 默认规则；`BBB_WASM_TARGET_DIR`、`BBB_WEB_OUT_DIR` 与 `BBB_WEB_CACHE_DIR` 用于 `cargo build-web` / `web-up` / E2E 脚本，独立启动的 `bbb-web-host` 读取 `BBB_WEB_OUT_DIR` 并支持 `--wasm-dir` / `--cache-dir` 覆盖）：

```bash
# 1. 准备官方 26.1 资源（若尚未准备）
cargo dev vanilla prepare 26.1

# 2. 构建多线程 WASM 客户端与 wasm-bindgen 产物（Linux / macOS / Windows 统一入口）
cargo build-web

# 3a. 单终端一键拉起本地 bbb-server + bbb-web-host（按一次 Ctrl+C 一起退出）
cargo dev web-up --world ../bbb-world

# 3b. 或仅单独启动网关并连接指定上游 26.1 离线模式服务器（可选开启反向 MCP 与 Control 端口）
cargo dev run -p bbb-web-host -- \
  --bind 127.0.0.1:8080 \
  --server 127.0.0.1:25565 \
  --allow-server play.example.org:25565 \
  --mcp-bind 127.0.0.1:40100 \
  --control 127.0.0.1:40101
```

在支持 WebGPU 的浏览器打开终端输出的地址。省略用户名时，每个浏览器配置文件会生成并保存自己的离线玩家名。使用同一个配置文件开两个标签页联机时，分别打开以下地址：

```text
http://127.0.0.1:8080/?username=Alex
http://127.0.0.1:8080/?username=Steve
```

### URL 查询参数、网关命令行参数、本地持久化与安全边界

浏览器地址栏支持下列已实现的 URL 查询参数（覆盖网关默认值；完整参数说明见[命令行与查询参数参考](cli-reference.md#web-客户端-url-查询参数)）：

- **连接与身份**：`?server=<host:port>`（必须在 `{--server} ∪ {--allow-server ...}` 白名单内，环回地址在同一允许端口上等价兼容 `127.0.0.1`、`localhost` 与 `[::1]`，表外请求 `/ws` 返回 `403 Forbidden`）、`?username=<name>`、`?uuid=<uuid>`。
- **渲染、画质与引擎扩展**：`?render_distance=<2..=32>`、`?fov=<30..=110>`、`?gui_scale=<0..=2147483646>`、`?graphics=<fast|fancy|fabulous|custom>`、`?improved_transparency=<bool>`、`?clouds=<off|fast|fancy>`、`?vsync=<bool>`、`?gamma=<0.0..=1.0>`、`?smooth_lighting=<bool>`、`?particles=<all|decreased|minimal>`、`?bob_view=<bool>`、`?fov_effect_scale=<0.0..=1.0>`、`?dynamic_lights=<off|fast|fancy>`、`?lod_distance=<0|16..=256>`、`?ext=<slug,...>`（支持重复或逗号分隔，如 `zoom,hud_info,item_inspector`）。由 URL 覆盖的选项独立于同源选项存储，当前页保存其他选项时保留这些字段的原有存储值。
- **语言、输入、旁白与音频**：`?lang=<code>`（别名 `?locale=<code>`）、`?sensitivity=<0.0..=1.0>`、`?subtitles=<bool>`、`?narrator=<off|all|chat|system|0..=3>`、`?volume_<category>=<0.0..=1.0>` 或 `?soundCategory_<category>=<0.0..=1.0>`（覆盖全部 11 个声音分类：`master`、`music`、`record`、`weather`、`block`、`hostile`、`neutral`、`player`、`ambient`、`voice`、`ui`）。
- **资源包策略与本地包**：`?server_resource_pack_policy=<prompt|accept|decline>`（别名 `?resource_pack_policy=`，默认 `prompt` 显示服务端说明）；`?pack=<url>`（可重复，挂载初始 `.zip` 资源包）。鼠标解除锁定时也可在页面右上角选择本地 `.zip` 或拖入画布，挂载触发原子重载；已挂载与读取中的本地 ZIP（含 `?pack=`）共享 256 MiB 限额，重复添加字节相同的包保留原有优先顺序，页面右上角可一键清除所有本地包并保留服务端资源包。
- **通用 `options.txt` 透传（`?opt.<key>=<value>`）**：可传递任意目录内已知
  `options.txt` 键及 `key_*` 按键绑定（如 `?opt.renderDistance=12`、
  `?opt.key_key.attack=key.mouse.right`、`?opt.soundCategory_music=0.25`、
  `?opt.modelPart_cape=false`）。启动前按类型与键值白名单校验，并仅对当前标签页在内存中
  合并生效（不写入浏览器同源 `localStorage["bbb.options.txt"]`，当前页保存其他选项时保留
  原有存储值），未知键或非法值拒绝启动；专用参数（如 `?fov=`、`?volume_<category>=`）仍生效。
- **控制、软件 GPU 与确定性步进**：`?control=1|true`（请求浏览器连接 `/control-ws` 反向隧道，需 `bbb-web-host` 已配置 `--mcp-bind` 或 `--control`）、`?software_gpu=1|true`（允许选择软件 WebGPU 适配器，供无显卡 CI / SwiftShader E2E）、`?lockstep=1|true`（虚拟时钟步进模式）。

`bbb-web-host` 命令行参数（详见[命令行参考](cli-reference.md#bbb-web-hostweb-网关)）：

- `--bind <addr>`（默认 `127.0.0.1:8080`）、`--server <host:port>`（默认 `127.0.0.1:25565`）、`--allow-server <host:port>`（可重复）。
- `--vanilla-root <path>`、`--wasm-dir <path>`（默认 `$BBB_WEB_OUT_DIR` 或 `<target-root>/bbb-web-dist`）、`--cache-dir <path>`（默认 `<target-root>/bbb-web-cache`）。
- 默认客户端启动值：`--username <name>`（单个标签页 `?username=` 优先）、`--locale <code>`、`--uuid <uuid>`、`--render-distance <2..=32>`、`--server-resource-pack-policy <prompt|accept|decline>`。
- 调试与代理开关：`--allow-pack-loopback`（允许 `/proxy/pack` 访问环回地址，仅限本地联调/测试）、`--allow-pack-private`（允许 `/proxy/pack` 访问自托管局域网 RFC 1918 / ULA 私网资源包，仍阻断环回/链路本地/云元数据）、`--mcp-bind <addr>`、`--control <addr>`（仅限环回地址）。

持久化与安全边界：

- 浏览器同源 `localStorage` 保存 `bbb.offline-player`（生成的离线玩家名）、`bbb.options.txt`（客户端选项）、`bbb.code_of_conduct.json`（已接受的服务器行为准则哈希）、`bbb.hotbar.nbt`（创造模式快捷栏预设）与 `bbb.servers.dat`（最近连接服务器列表）。
- 远端断开后，断开界面的 **重新连接（Reconnect）** 会刷新当前页面并连接到同一启动地址。
- `/proxy/skin` 仅允许 `textures.minecraft.net`、`api.mojang.com`、`sessionserver.mojang.com` 与 `api.minecraftservices.com`。
- `/proxy/pack` 限制下载体积（256 MiB）与重定向次数，并在 DNS 解析阶段校验目标 IP，默认拒绝环回、私网与链路本地地址（完整威胁模型与反代清单见[安全边界与生产部署指南](security-and-deployment.md)）。

## 资源分层与缓存

1. **核心包 `core-26.1.zip`（约 14–19 MiB）与 WASM 载荷（`pkg/bbb_client_bg.wasm` 原始约
   22–29 MiB，gzip 传输约 9 MiB）**：`bbb-web-host` 的 `--vanilla-root` 支持三种 26.1 资产布局（已解包工作区 `sources/26.1` + `downloads/26.1`、下载缓存 `downloads/26.1/{client-26.1.jar,client.jar,26.1.jar}`，或官方 `.minecraft` 目录 `versions/26.1/` + `assets/`；未显式传入时依次探测 `BBB_VANILLA_ROOT`、`XDG_CACHE_HOME/bbb/vanilla` 与平台托管缓存，不自动探测 `.minecraft`）。启动时网关将 `version.json`、合并了
   26.1 `pack` 格式元数据的 `pack.mcmeta`、客户端 `assets/**`（省略 `programmer_art` /
   `high_contrast` 内置可选包）、客户端所需的 `data/**`（同步注册表 JSON、
   `tags/*.json` 与内置数据包 `pack.mcmeta`/同步注册表条目）以及非音频索引资产
   （`minecraft/sounds.json`、`minecraft/font/*`、`minecraft/textures/*`、`icons/*`
   与内置中英文语言包）打包并缓存在 `--cache-dir`，响应附带 SHA-1 `ETag` 与
   `Content-Length`。浏览器按流分块读取并在加载界面显示实时下载进度（MiB 与百分比），
   首次拉取后存入 Cache Storage（`bbb-core-assets-v1`），后续刷新命中 `ETag` / 本地缓存。
2. **按需语言包与 RTL**：除内置中英文外，其余约 120 种 Vanilla 语言包（含 RTL 语言）通过 `/assets/asset-index.json` 解析索引，按 `?locale=` 或保存的语言选项经 `/assets/objects/<2hex>/<40hex>` 按需拉取（网关返回 `Cache-Control: immutable` 供浏览器 HTTP 缓存复用）、校验 SHA-1 与字节长度并挂载至内存 `virtual_store`。
3. **按需音频对象与离主线程解码**：约 4500 个 `.ogg` 音频对象不打入首包，仅在声音事件触发时异步拉取、核验 SHA-1 与长度并写入 Cache Storage（`bbb-audio-v1`），再经浏览器原生 `AudioContext::decodeAudioData` 离主线程异步解码为 `AudioBuffer`。

## 反向 MCP 与端到端验证

当 `bbb-web-host` 传入 `--mcp-bind` 或 `--control` 时，活跃浏览器标签页通过 `/control-ws` 建立反向控制隧道：
宿主机侧直接复用 `bbb_mcp::control::serve_remote` 与 `bbb_mcp::control::serve_remote_control`，无需每 tick 序列化 `WorldStore`；查询、按键/视角控制与 `renderer.screenshot` 转发至浏览器内权威 `ControlState` 执行，异步 WebGPU 读回的 PNG 截图回传并落盘到宿主机请求路径。

定向与端到端验证命令：

```bash
# 网关协议桥接、ETag/COOP/COEP 头、路径穿越拒绝与反向控制隧道集成测试
cargo dev test -p bbb-web-host

# 真实 bbb-server + bbb-web-host + 本机 Chromium WebGPU 进服、渲染、移动、MCP 与截图门禁
cargo dev e2e web

# 经反向 MCP 隧道运行 S1–S6 多场景自主游玩闭环验证（生存合成、熔炼、战斗、交易、建造与重生恢复）
cargo dev python scripts/mcp_agent_scenarios.py --mode policy --client web --out /tmp/bbb-mcp-scenarios-web
```

## 浏览器与平台能力边界

- **跨源隔离与安全上下文（COOP/COEP）**：多线程 WASM 网格烘培（`SharedArrayBuffer` + `wasm-bindgen-rayon` Web Worker）与 WebGPU 要求页面运行在浏览器安全上下文（`https://` 或 `http://localhost` / `http://127.0.0.1`），并携带 `Cross-Origin-Opener-Policy: same-origin`、`Cross-Origin-Embedder-Policy: require-corp` 与 `Cross-Origin-Resource-Policy: same-origin` 响应头；反向代理部署配置见 [`packaging/README.md`](../../../packaging/README.md)。
- **浏览器用户手势与网络沙箱**：Pointer Lock 鼠标捕获、全屏切换（`F11`）、Web Audio（`AudioContext` 恢复播放）、Web Speech 旁白（`Ctrl+B`）与异步剪贴板读写受浏览器用户激活策略约束；CJK 输入法仅在文本输入界面通过隐藏 `#bbb-ime-input` 接管合成事件，且不拦截 `F12` / `Ctrl+Shift+I` / `Cmd+Option+I` 开发者工具快捷键。浏览器无法直接发起原始 Minecraft TCP 连接，连接统一经 `bbb-web-host` 的 `/ws` 白名单二进制桥接转发。
- **桌面原生专有能力边界**：原生 MP4 视频录制（`crates/bbb-client/src/recording.rs`，基于 Windows Media Foundation 的 `F9` / `--record-video` / `renderer.recording_start`）仅支持 Windows 原生客户端，在 `wasm32` 目标上调用会明确返回不支持；`F3+S` 动态纹理图集转储（桌面写入 `<game-dir>/screenshots/debug`）在 `wasm32` 上不启用；浏览器端 `F2` 截图与 `F3+L` 性能剖析报告（`.zip`）均通过 Blob 触发浏览器下载，反向隧道 `renderer.screenshot` 则将 PNG 写回 `bbb-web-host` 宿主机路径。宿主机本地 `crash-reports/` 目录直接写盘在浏览器沙箱内不启用（崩溃与诊断信息输出至浏览器控制台与页面覆盖层）。
