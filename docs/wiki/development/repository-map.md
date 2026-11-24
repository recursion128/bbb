# 仓库地图

## 产品 crate

| 路径 | 语义所有者 | 主要输出 |
| --- | --- | --- |
| `crates/bbb-facts` | 生成的 26.1 事实与目录表（含 `default_attributes_26_1` 与 `fuel_burn_times`）、无运行时纯查询、两端共享的扫掠 AABB 碰撞与射线裁切（`collision`）及爆炸数学（`explosion`），零工作区依赖 | 生成的事实表、纯查询、`collision`、`explosion` |
| `crates/bbb-protocol` | 双向 26.1 协议线层编解码器、数据包目录、文本组件/NBT、`profile_resolution`、两端共享的纯光照存储/传播（`chunk_light`）与生物体转向（`entity_body_rotation`）；仅依赖 `bbb-facts` | 强类型数据包、标识符、`chunk_light` |
| `crates/bbb-registry-codec` | 注册表元素编解码器、持久化组件/物品 NBT 编解码器、随机源、群系气候；仅依赖 `bbb-facts` 与 `bbb-protocol`（不依赖 `bbb-render-types`） | 已解码注册表元素、持久化 NBT 投影 |
| `crates/bbb-render-types` | 跨客户端与渲染器的窄渲染值类型及共享 WASM 同步原语（`wasm_sync`）；仅依赖 `bbb-facts` 与 `bbb-protocol` | 帧渲染原语、`wasm_sync` |
| `crates/bbb-client-net` | 客户端 TCP/WebSocket 传输、连接阶段、收发顺序、会话录制、Mojang Services 公钥与异步玩家资料/皮肤 HTTP 服务（`profile_textures`）；仅依赖 `bbb-client-clock` 与 `bbb-protocol`（与世界和资源解耦） | 有序客户端网络事件、玩家资料与皮肤拉取 |
| `crates/bbb-server-net` | 服务端监听器、连接阶段（含 `SelectKnownPacks` 与 Configuration 重入）、协议压缩、预编码广播帧与有界交接队列 | 有序服务端连接事件与命令 |
| `crates/bbb-server-types` | 错误/存档（含 `save::world_gen`）/物品/几何/交互、纯方块查询与解析、玩家经验（`experience`）/槽位范围（`slot_ranges`）/统计/进度文件值（`advancements`）/食物与伤害值、流体投影、幽匿（sculk）/随机序列保存值、区块调色板与高度图（`chunk_data`）及计划刻容器 | 无宿主依赖的服务端共享值与查询 |
| `crates/bbb-server-datapack` | 数据包（datapack）加载、动态注册表（`/reload` 保持启动冻结注册表）、世界生成强类型定义、预设与平坦/调试生成源（`flat_generator`、`debug_generator`）、伤害/食物定义、玩家进度树与内嵌 Vanilla 快照 | 经校验的数据代际快照 |
| `crates/bbb-server-worldgen` | 地物（feature）、噪声与密度函数、雕刻器、结构、群系源与确定性生成随机源 | 世界生成算法；服务端生产依赖仅 `bbb-server-types` 与 `bbb-server-datapack`（另共享 `bbb-facts`、`bbb-protocol`、`bbb-registry-codec`） |
| `crates/bbb-server-persistence` | 26.1 Anvil/区域/世界/玩家/实体/兴趣点（POI）存档结构、`SavedData`、`atomic_file` 与 `session_lock`；与 `bbb-server-worldgen`、`bbb-server-datapack` 及玩法完全解耦 | 存档读写，不依赖世界生成、数据包与玩法 |
| `crates/bbb-server-core` | 方块/物品/机器、区块/光照、实体/AI（扫掠碰撞复用 `bbb_facts::collision`）、玩家/会话、世界创建（`world_creation`）、tick、命令、管理、邮箱、`registry_sync` 与同步 | 唯一服务端玩法所有者 |
| `crates/bbb-server` | 无头命令行入口、连接接收与任务调度、预编码广播路由、20 Hz 世界驱动接线、系统信号与优雅停机 | 独立服务端可执行程序与运行时 |
| `crates/bbb-mcp` | 两端共用的零工作区依赖 MCP 传输层：Streamable HTTP、JSON-RPC、会话、鉴权/OAuth/TLS 与远程控制隧道（`control`），对 `McpHost` 与权限阶梯保持泛型 | MCP 端点骨架 |
| `crates/bbb-server-admin` | `AdminHost`：把 `AdminRequest` 工具目录与权限范围接进 `bbb-mcp` | 管理端点 |
| `crates/bbb-admin` | 管理端点的轻量命令行管理客户端，生产仅依赖 `bbb-mcp`（零服务端 crate 依赖） | `bbb-admin` 命令行工具 |
| `crates/bbb-client-clock` | 零工作区依赖的注入式客户端与回放时钟 | 经过时间与当前时间源 |
| `crates/bbb-client-assets` | 资源包栈、资产索引与对象存储、`virtual_store`、资源代际管理 | 经校验的客户端资源 |
| `crates/bbb-client-world` | 权威客户端世界、实体、区块与界面状态；本地预测移动碰撞（复用 `bbb_facts::collision`）、ICU 本地时间（`local_time`）、方块轮廓几何（`block_outline`）、坐标/调色板/区块 NBT 原语与方块状态/元素值目录 | 查询、状态与特效输入 |
| `crates/bbb-client-render` | GPU 管线与渲染原语（`render_core`）、各渲染通道、物品/字体/模型烘焙（`font::ttf` 用纯 Rust `ab_glyph`）与场景纹理投影；与 `bbb-client-world`/HTTP/FFI/`icu_*` 解耦 | 呈现帧与渲染计数器 |
| `crates/bbb-client-audio` | 原生（OpenAL 生产后端；`kira` 仅限测试）与浏览器（`webaudio` 离主线程解码）音频后端及声道生命周期 | 播放观测结果 |
| `crates/bbb-client-probe` | 客户端无头差分/会话 `probe` 测试框架与回放截图比对工具 `bbb-replay-diff`；依赖 `bbb-client-assets`、`bbb-client-net`、`bbb-client-world` 与 `bbb-protocol` | 会话探针库、`bbb-replay-diff` 工具 |
| `crates/bbb-client` | 命令行入口、桌面/WASM 入口（`platform_macos`、`web`）、客户端模拟、`terrain_upload`、`tooltip`、`options`/`server_list`/`hotbar_presets`/`code_of_conduct` 持久化、`control` 与 `mcp` | `bbb-client` 与 WASM `cdylib` |
| `crates/bbb-web-host` | Web 客户端同源 COOP/COEP 宿主、WS↔TCP 桥、皮肤/资源包代理与反向 MCP/control 网关；零 `bbb-client`/`wgpu`/`winit` 依赖 | `bbb-web-host` 可执行网关 |
| `crates/bbb-xtask` | 零工作区依赖的跨平台开发与测试执行器（`cargo dev`、`cargo worker`、`cargo build-web`、`cargo web-up`） | `bbb-xtask` 开发执行器 |

完整 24-crate 生产依赖有向无环图（DAG）与边界方向统一维护在[总体架构 DAG](../architecture/overview.md#workspace-24-crate-生产依赖有向无环图dag)：底层 crate 不反向读取运行时或渲染器状态，跨 crate API 表达语义，不为拆文件放宽 `pub` 可见性。

## 独立服务端边界

服务端 crate 为 `bbb-server-net`、`bbb-server`、`bbb-server-admin`、`bbb-admin` 与权威
世界/数据的五个 crate（数据与行为的依赖关系见
[服务端边界](../architecture/server-boundaries.md)）。世界生成的服务端生产依赖仅指向 `bbb-server-types` 与 `bbb-server-datapack`（另共享 `bbb-facts`、`bbb-protocol` 与 `bbb-registry-codec`），
实时方块和实体玩法直接借用核心层的 `LevelCtx`；形状更新的 `BlockShapeCtx` 共用实时、生成和升级规则，
持久化层不依赖玩法 crate：

| 语义所有者 | 负责内容 | 复用与禁止事项 |
| --- | --- | --- |
| `bbb-server-net` | 监听器、连接阶段、压缩与有界连接队列 | 复用 `bbb-protocol`；不反转 `bbb-client-net` 客户端事件状态 |
| `bbb-server-core` 与独立数据 crate | tick、维度/区块/实体/玩家、玩法、数据包与 26.1 持久化（按语义模块拆分） | 复用中立生成事实；不复用 `bbb-client-world` 实时状态，不依赖渲染器、音频或 `winit` |
| `bbb-server` | 命令行入口、连接路由、20 Hz 世界集成、信号处理、崩溃与无头接线 | Linux/macOS 正式无头发布二进制（Windows 支持本地开发与 E2E）；无图形界面或标准输入命令所有者 |
| `bbb-mcp` / `bbb-server-admin` / `bbb-admin` | 共享 MCP 传输、强类型管理投影、鉴权与审计、命令行管理客户端 | `bbb-mcp` 不依赖任何世界或客户端 crate；管理端不直接借用世界状态，普通查询与写入经有界 tick 邮箱；不建第二套命令分派器 |

数据包与持久化层已由独立 crate 拥有生命周期、窄 API 和聚焦测试；
玩法已并入核心层；运行时从核心层的世界、邮箱、连接等实际模块消费 API。
数据包与持久化层保持独立的数据生命周期，运行时直接依赖实际数据所有者；
核心层根模块不重导出其他数据 crate 或汇总所有玩法符号。完整不变量见
[独立服务端边界](../architecture/server-boundaries.md)。

## 工具与生成事实

`crates/bbb-xtask` 拥有跨平台日常构建、测试、worker 隔离与完整门禁命令行入口；`scripts/` 保存 Python
源码审计（`scripts/audit_p0.py`、`scripts/docs_lint.py`）、事实生成器与 POSIX 真机门禁。POSIX
兼容入口是 `scripts/dev`、`scripts/vanilla`、`scripts/process` 与 `scripts/input`；实现归
`scripts/bbb_tools` 下按 `vanilla`、`execution`、`host`、`input` 分域，测试归 `scripts/tests`，锁文件归
`scripts/data`。顶层入口不拥有业务逻辑；
`scripts/fixtures/` 保存小型、可评审且无网络的测试夹具。`docs/evidence/vanilla-26.1/*.json` 保存 26.1
生成的事实目录，不保存外部 JAR、资产或抓取录像。各 crate 的集成测试目录（`crates/*/tests/`）和源码内
`#[cfg(test)]` 按所有者放置聚焦测试。

独立服务端可玩门禁与归一化判据统一由 `scripts/server_playable_gate.py`
（`cargo dev e2e server`）拥有，发布验收族由 `scripts/server_release_gate.py`
（`cargo dev e2e server-release`）拥有；它们复用仓库管理的 Java/JAR 和 `bbb-client --probe-server`（由 `bbb-client-probe` 承载），
不成为另一套协议或世界所有者。

## 文档

[`docs/README.md`](../../README.md) 是总入口，Wiki 保存长期事实。[`AGENTS.md`](../../../AGENTS.md)、账本（ledger）与 Git 的
权威边界见[文档工作流](documentation-workflow.md)。全部 Markdown 处理结论见
[文档清单](document-inventory.md)。
