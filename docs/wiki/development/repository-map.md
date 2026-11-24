# 仓库地图

## 产品 crate

| 路径 | semantic owner | 主要输出 |
| --- | --- | --- |
| `crates/bbb-facts` | 生成的 26.1 事实/catalog 表（含 `default_attributes_26_1` 与 `fuel_burn_times`）、无运行时纯查询、两端共享的扫掠 AABB 碰撞与射线裁切（`collision`）及爆炸数学（`explosion`），零 workspace 依赖 | generated fact tables、pure lookups、`collision`、`explosion` |
| `crates/bbb-protocol` | 双向 26.1 wire codec、packet catalog、Component/NBT、`profile_resolution`、两端共享的纯光照存储/传播（`chunk_light`）与生物体转向（`entity_body_rotation`）；仅依赖 `bbb-facts` | typed packets、ids、`chunk_light` |
| `crates/bbb-registry-codec` | registry element codec、persistent component/item NBT codec、random source、biome climate；仅依赖 `bbb-facts` 与 `bbb-protocol`（不依赖 `bbb-render-types`） | decoded registry elements、persistent NBT projections |
| `crates/bbb-render-types` | 跨客户端与渲染器的窄渲染值类型及共享 WASM 同步原语（`wasm_sync`）；仅依赖 `bbb-facts` 与 `bbb-protocol` | frame primitives、`wasm_sync` |
| `crates/bbb-client-net` | 客户端 TCP/WebSocket 传输、连接 phase、收发顺序、会话录制、Services 公钥与异步 Mojang profile/skin HTTP 服务（`profile_textures`）；仅依赖 `bbb-client-clock` 与 `bbb-protocol`（与 world/assets 解耦） | ordered client network events、profile/skin fetch |
| `crates/bbb-server-net` | 服务端 listener、连接 phase（含 `SelectKnownPacks` 与 Configuration 重入）、compression、预编码广播帧与有界 handoff | ordered server connection events/commands |
| `crates/bbb-server-types` | 错误/save（含 `save::world_gen`）/物品/几何/interaction、纯方块查询/解析、玩家经验（`experience`）/槽位范围（`slot_ranges`）/统计/进度文件值（`advancements`）/食物与伤害值、流体投影、sculk/random sequence 保存值、区块调色板与高度图（`chunk_data`）及计划刻容器 | 无 host 的服务端共享值与查询 |
| `crates/bbb-server-datapack` | datapack 加载、dynamic registry（`/reload` 保持启动冻结注册表）、worldgen 强类型定义、预设与平坦/调试生成源（`flat_generator`、`debug_generator`）、damage/food、玩家进度树与内嵌 vanilla snapshot | validated data generation |
| `crates/bbb-server-worldgen` | feature、noise/density、carver、structure、biome source 与确定性生成随机源 | 生成算法；服务端生产依赖仅 `bbb-server-types` 与 `bbb-server-datapack`（另共享 `bbb-facts`、`bbb-protocol`、`bbb-registry-codec`） |
| `crates/bbb-server-persistence` | 26.1 Anvil/region/level/player/entity/POI save schema、SavedData、`atomic_file` 与 `session_lock`；与 `bbb-server-worldgen`、`bbb-server-datapack` 及玩法完全解耦 | 存档读写，不依赖 worldgen/datapack/玩法 |
| `crates/bbb-server-core` | 方块/物品/机器、区块/光照、实体/AI（扫掠碰撞复用 `bbb_facts::collision`）、玩家/session、世界创建（`world_creation`）、tick、命令、管理、mailbox、`registry_sync` 与同步 | 唯一服务端玩法 owner |
| `crates/bbb-server` | headless CLI、accept/task、预编码广播路由、20 Hz world wiring、signal、graceful shutdown | dedicated server binary/runtime |
| `crates/bbb-mcp` | 两端共用的零 workspace 依赖 MCP 传输层：Streamable HTTP、JSON-RPC、session、auth/OAuth/TLS 与远程控制隧道（`control`），对 `McpHost` 与 scope 梯子泛型 | MCP endpoint 骨架 |
| `crates/bbb-server-admin` | `AdminHost`：把 `AdminRequest` tool 目录与 scope 接进 `bbb-mcp` | 管理 endpoint |
| `crates/bbb-admin` | 管理 endpoint 的轻量 human CLI 客户端，生产仅依赖 `bbb-mcp`（零服务端 crate 依赖） | `bbb-admin` CLI |
| `crates/bbb-client-clock` | 零 workspace 依赖的注入式客户端与回放时钟 | elapsed/now |
| `crates/bbb-client-assets` | pack stack、asset-index/object-store、`virtual_store`、resource generation | validated resources |
| `crates/bbb-client-world` | canonical client world、entity、chunk、UI state；本地预测移动碰撞（复用 `bbb_facts::collision`）、ICU 本地时间（`local_time`）、方块轮廓几何（`block_outline`）、position/palette/chunk NBT 原语与 block-state/element-value 目录 | query/state/effects inputs |
| `crates/bbb-client-render` | GPU 管线与渲染原语（`render_core`）、各渲染 pass、item/font/model bake（`font::ttf` 用纯 Rust `ab_glyph`）与场景纹理投影；与 `bbb-client-world`/HTTP/FFI/`icu_*` 解耦 | presented frame/counters |
| `crates/bbb-client-audio` | 原生（OpenAL 生产后端；`kira` 仅限测试）与浏览器（`webaudio` 离主线程解码）audio backend、voice 生命周期 | playback observations |
| `crates/bbb-client-probe` | 客户端无头差分/会话 `probe` harness 与回放截图比对工具 `bbb-replay-diff`；依赖 `bbb-client-assets`、`bbb-client-net`、`bbb-client-world` 与 `bbb-protocol` | session probe library、`bbb-replay-diff` CLI |
| `crates/bbb-client` | CLI、桌面/WASM 入口（`platform_macos`、`web`）、simulation、`terrain_upload`、`tooltip`、`options`/`server_list`/`hotbar_presets`/`code_of_conduct` 持久化、`control` 与 `mcp` | `bbb-client` 与 WASM `cdylib` |
| `crates/bbb-web-host` | Web 客户端同源 COOP/COEP 宿主、WS↔TCP 桥、皮肤/资源包代理与反向 MCP/control 网关；零 `bbb-client`/`wgpu`/`winit` 依赖 | `bbb-web-host` 可执行网关 |
| `crates/bbb-xtask` | 零 workspace 依赖的跨平台开发与测试 runner（`cargo dev`、`cargo worker`、`cargo build-web`、`cargo web-up`） | `bbb-xtask` 开发执行器 |

完整 24-crate 生产依赖有向无环图（DAG）与边界方向统一维护在[总体架构 DAG](../architecture/overview.md#workspace-24-crate-生产依赖有向无环图dag)：底层 crate 不反向读取 runtime 或 renderer 状态，跨 crate API 表达语义，不为拆文件暴露 broad `pub`。

## Dedicated server 边界

服务端 crate 为 `bbb-server-net`、`bbb-server`、`bbb-server-admin`、`bbb-admin` 与权威
world/data 的五个 crate（数据与行为的依赖关系见
[server 边界](../architecture/server-boundaries.md)）。Worldgen 的服务端生产依赖仅指向 Types 与 Datapack（另共享 `bbb-facts`、`bbb-protocol` 与 `bbb-registry-codec`），
实时方块和实体玩法直接借用 Core 的 `LevelCtx`；形状更新的 `BlockShapeCtx` 共用实时、生成和升级规则，
Persistence 不依赖玩法 crate：

| 语义 owner | 拥有 | 复用/禁止 |
| --- | --- | --- |
| `bbb-server-net` | listener、phase、compression、bounded connection queues | 复用 `bbb-protocol`；不反转 `bbb-client-net` client event state |
| `bbb-server-core` 与独立数据 crate | tick、dimension/chunk/entity/player、玩法、datapack、26.1 persistence（按语义模块拆分） | 复用中立 generated facts；不复用 `bbb-client-world` live state，不依赖 renderer/audio/winit |
| `bbb-server` | CLI、connection route、20 Hz world integration、signal、crash、headless wiring | Linux/macOS 正式无头发布二进制（Windows 支持本地开发与 E2E）；无 GUI 或 stdin command owner |
| `bbb-mcp` / `bbb-server-admin` / `bbb-admin` | 共享 MCP 传输、typed admin projection、auth/audit、human CLI | `bbb-mcp` 不依赖任何 world/client crate；admin 不直接 borrow world，普通查询与写入经 bounded tick mailbox；不建第二 command dispatcher |

datapack 与 persistence 已由独立 crate 拥有生命周期、窄 API 和 focused tests；
玩法已并入 Core；运行时从 Core 的 world、mailbox、connection 等实际模块消费 API。
Datapack 与 Persistence 保持独立的数据生命周期，runtime 直接依赖实际数据 owner；
Core 根不重导出其它数据 crate 或汇总所有玩法符号。完整不变量见
[Dedicated server 边界](../architecture/server-boundaries.md)。

## 工具与生成事实

`crates/bbb-xtask` 拥有跨平台日常构建、测试、worker 隔离与 full gate CLI；`scripts/` 保存 Python
source audit（`scripts/audit_p0.py`、`scripts/docs_lint.py`）、generator 与 POSIX live 门禁。POSIX
兼容入口是 `scripts/dev`、`scripts/vanilla`、`scripts/process` 与 `scripts/input`；实现归
`scripts/bbb_tools` 下按 vanilla、execution、host、input 分域，测试归 `scripts/tests`，锁归
`scripts/data`。顶层入口不拥有业务逻辑；
`scripts/fixtures/` 保存小型、可评审且无网络的 fixture。`docs/evidence/vanilla-26.1/*.json` 保存 26.1
generated catalog，不保存外部 jar/assets/capture。各 crate 的集成测试目录（`crates/*/tests/`）和源码内
`#[cfg(test)]` 按 owner 放置 focused tests。

dedicated server 可玩门禁与归一化判据统一由 `scripts/server_playable_gate.py`
（`cargo dev e2e server`）拥有，发布验收族由 `scripts/server_release_gate.py`
（`cargo dev e2e server-release`）拥有；它们复用 repo-managed Java/jar 和 `bbb-client --probe-server`（由 `bbb-client-probe` 承载），
不成为另一套 protocol/world owner。

## 文档

[`docs/README.md`](../../README.md)是总入口，Wiki 保存长期事实。`AGENTS.md`、ledger 与 Git 的
权威边界见[文档工作流](documentation-workflow.md)。全部 Markdown 处理结论见
[inventory](document-inventory.md)。
