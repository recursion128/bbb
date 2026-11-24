# 开发流程、测试与发布

## 原则

流程只为更快交付可用功能服务。验证证明玩家或存档可观察的结果，成本按风险支付；不为仪式、
收据或状态机付时间。推进原则（协议严格、只读官方存档、功能基本对齐、实现自由、代码更少）见
[AGENTS.md](../../../AGENTS.md)。

日常构建、测试、审计、worktree 协作、E2E 与 full gate 在 Windows、macOS、Linux 上统一由零外部依赖的
`crates/bbb-xtask`（Cargo 别名 `cargo dev`、`cargo worker`、`cargo build-web`、`cargo web-up`）拥有，
不维护双份 Bash/PowerShell 日常脚本。测试体系按速度与运行依赖分为五层（L0–L4），覆盖
**Server + Web + Windows Client + Linux Client** 四条固定测试线：

1. **L0 静态检查与多目标编译**：`cargo dev lint [--skip-vanilla]`（`fmt --check`、`git diff --check`、
   `audit-p0`、`py-lint`、`docs-lint`、`anchors`、`clippy --workspace --all-targets`）与 `cargo dev check-targets`（宿主 `--workspace --all-targets`、
   Web `wasm32-unknown-unknown`、第二桌面平台 `cross-check`），零外部资产即可在任意标准 CI Runner 上执行。
2. **L1 单元与离线集成测试**：`cargo dev test` / `cargo dev fast-test` / `cargo dev py-test`。所有默认（非 `#[ignore]`）
   Rust 测试均可离线运行；完整服务端数据包快照内嵌于仓库，需要本地 Mojang 26.1 资产或反编译源码的校验在缺失时干净跳过，
   在 `BBB_REQUIRE_VANILLA=1`（如 `cargo dev gate`）下强制执行。
3. **L2 跨端端到端（E2E）可玩门禁**：统一通过 `cargo dev e2e <server|web|client|server-release>` 调度，
   同时支持本机硬件 GPU 与 GitHub Actions 标准无显卡云 Runner（`--software-gpu` / `BBB_SOFTWARE_GPU=1`：
   Linux Weston + Lavapipe、Windows D3D12 WARP、Headless Chromium SwiftShader WebGPU）。
4. **L3 完整门禁（Full Gate）**：`cargo dev gate`（14 步），里程碑结束或发布前支付。
5. **L4 发布压力与物理硬件验收**：20 人并发/重启/soak（`cargo dev e2e server-release`）、桌面长稳与输入栈门禁、
   物理 GPU 性能基准，只在改动触及对应表面或发布时支付。

Vanilla 源码只确认功能事实，不证明 BBB 接线；截图“看起来接近”或 unit test 不能冒充真实平台通过。

## 日常循环

1. **选题**：用户指定优先；否则从 owning ledger 选玩家影响最大、解锁最多的功能缺口。P0 回退
   全局抢占。不需要运行 preflight 脚本或登记 owner。
2. **实现**：在 bbb 自身语义边界内写最简单可行的实现；能删掉过度对齐代码时一并删。
3. **验证**：feature 完成后，只有行为、数据格式、共享状态或已知热点的实质变化确实可能造成
   回退时，才选必要的定向验证。范围由实际风险决定，不因涉及某个 crate 或修改文件而扩大。
4. **提交与落地**：review 完整 diff 后直接提交到 primary（见[落地](#落地)），同一提交更新
   ledger（关闭即删除，剩余部分改写）与必要 Wiki。
5. **继续**：连续产品目标在落地后直接选下一项；有界任务完成即停。里程碑结束跑一次 full gate，
   红了先修。

按完整功能组织实现和提交，不按 helper 切碎，也不为凑批次合并无关改动。简单收敛改动不例行
支付检查、审计或测试；已通过且未受影响的结果直接复用，不增加多轮审查和报告。

## 测试选择

- 测试断言玩家/存档可观察的结果：packet 同步、规则/常量/掉落结果、读入官方存档后的状态、
  红石机器能工作、同一 BBB 种子与生成输入的地形及结构自身确定且符合生成机制。优先端到端（真实 packet → tick → 结果）
  而非逐 helper 断言。
- 不写锁定 tick 内部顺序、RNG 流、goal 相位、对象身份、回滚、Codec 错误路径或内部诊断文本的测试；
  删除过度对齐代码时同时删除只验证这些非目标的测试。
  命令报错和反馈文本属于玩家可见结果，继续验收。
- 优先复用已有测试；缺验收时只补最少回归。不枚举字段类型、极值、预算或异常组合，不为同一
  行为铺多层套件。边界回归仅限用户要求或本次缺陷直接涉及的边界。
- 测试数量和提交数不是进展；进展是 ledger 功能项关闭。

## 跨平台统一命令

以下命令在 PowerShell、Bash、Zsh 下完全一致：

| 场景 | 统一命令 |
| --- | --- |
| 新环境初始化与主机探测 | `cargo dev bootstrap [--skip-vanilla] [--offline]`、`cargo dev doctor [--profile <name>]`、`cargo dev env` |
| 快速静态与文档检查（L0） | `cargo dev lint [--skip-vanilla]`、`cargo dev clippy [cargo args...]`、`cargo dev fmt [--check]` |
| 全目标编译检查（L0） | `cargo dev check-targets`（宿主 + `wasm32-unknown-unknown` + 第二桌面平台 `cargo dev cross-check`） |
| 定向构建与类型检查 | `cargo dev build [cargo args...]`、`cargo dev check -p <crate> --all-targets` |
| 定向 Rust 测试（L1） | `cargo dev test -p <crate> <filter>` 或 `cargo dev fast-test -p <crate> <filter>` |
| Python 工具运行、测试与 Lint（L1） | `cargo dev python <cmd-or-script>`、`cargo dev py-test <path-or-node-id>`、`cargo dev py-lint` |
| 性能基准（Criterion / 服务端 / 回放） | `cargo dev bench [--list \| --check \| <name> \| server \| replay]` |
| 服务端可玩 E2E（L2） | `cargo dev e2e server`（与官方 26.1 差分：`BBB_PLAYABLE_ACCEPT_EULA=1` 下跑 `cargo dev e2e server live`） |
| Web 客户端 WebGPU E2E（L2） | `cargo dev e2e web`（无显卡云 Runner 追加 `--software-gpu`；本机开窗调试追加 `--headed`） |
| 桌面客户端 MCP + 回放 E2E（L2） | `cargo dev e2e client`（Linux 自动起 headless Weston，Windows 原生窗口；无显卡云 Runner 追加 `--software-gpu`） |
| 服务端发布族验收（L4） | `cargo dev e2e server-release [FAMILY ...]` |
| 运行程序与 MCP 本地建图 | `cargo dev run -p bbb-server -- --bind 127.0.0.1:25565`、`cargo dev world-build [options]`、`cargo dev cargo <args...>` |
| 桌面客户端构建与启动 | `cargo dev client [--release] [--] [args...]`（自动注入 `BBB_VANILLA_ROOT` 并透传 `bbb-client` 启动参数） |
| Web 客户端构建 / 一键联调 | `cargo build-web`、`cargo web-up --vanilla-root <dir>` |
| Vanilla 工作区与生成目录检查 | `cargo dev vanilla path 26.1`、`cargo dev vanilla verify 26.1`、`cargo dev generate --check [name]` |
| 单项文档、P0 与锚点检查 | `cargo dev docs-lint [--all-generators] [--skip-vanilla]`、`cargo dev audit-p0`、`cargo dev anchors [--all]` |
| 完整/便携门禁与二分定位（L3） | `cargo dev gate [--dry-run] [--skip-vanilla]`、`cargo dev batch-gate [<good>..<bad>]` |
| 缓存体积与清理 | `cargo dev size`、`cargo dev prune [--force]`、`cargo dev clean-target <name>` |
| 提交前检查与钩子安装 | `cargo dev pre-commit`、`cargo dev install-hooks` |

- `cargo test` 在同一测试二进制内以多线程共享进程运行，datapack/世界夹具每个二进制只建一次；
  深栈配置 `RUST_MIN_STACK` 与共享 `CARGO_TARGET_DIR` 由 `.cargo/config.toml` 和 `cargo dev` 自动持有。
- filter 按 owner 模块限定，避免宽匹配带入昂贵场景。target 目录与并行 worker 租约见[构建与缓存](build-and-cache.md)。
- 不跑 `cargo fmt` 全仓重写，不临时设 `RUSTFLAGS='-D warnings'`，不生成 repo-local `target`。

## 落地

- **solo**：主 agent 在 primary 工作树 review 完整 diff，完成适用的必要验证后直接提交，
  历史线性、只 fix-forward。改动 Vanilla 行号锚点时跑 `cargo dev anchors`。
- **并行**：至少两个任务路径不重叠、接口稳定且可分别验收时才开 worker。
  `cargo worker create <name>` 建 worktree `../bbb-wt-<name>`、分支 `worker/<name>` 与暖池 target 租约；
  定向验证用 `cargo worker run <name> -- cargo dev test -p <crate> <filter>`。
  worker 只在自己分支提交，落地前把自己的分支 rebase 到 primary。主 agent review 后在 primary
  `git merge --ff-only worker/<name>`，然后 `cargo worker cleanup <name>` 回收。
  共享文件由主 agent 指定单一编辑者；不能用 Git 自动合并代替语义 review。

## Full gate

`cargo dev gate` 每次无条件依次执行 14 步：Vanilla verify、host doctor、docs-lint（`scripts/docs_lint.py --all-generators`）、
`cargo fmt --all --check`、Vanilla anchors（`scripts/check_vanilla_anchors.py --all`）、P0 audit（`scripts/audit_p0.py`）、
Python lint、Python tests（`--suite all`）、`git diff --check`、`cargo clippy --workspace --all-targets`（`-D warnings`）、Web `wasm32-unknown-unknown` check、
second-platform cross-check（`scripts/cross_check.py`）、workspace `cargo test`（强制 `BBB_REQUIRE_VANILLA=1`）
与 doc tests，不按改动路径跳步、不复用收据；通过后在
`<git-common-dir>/bbb/gate-log.tsv` 记录分步耗时，并在 clean 且未移动的 HEAD 上更新 `last-gated`。
无本地 Vanilla 26.1 缓存时可运行 `cargo dev gate --skip-vanilla` 执行 12 步便携门禁（跳过 Vanilla verify 与
anchors，`docs-lint` 传 `--skip-vanilla`，Python 测试跑 `--suite portable`，仅记录 `mode:portable` 而不推进 `last-gated`）。
失败用 `cargo dev batch-gate [<good>..<bad>]` 在临时无头 worktree 中二分定位首个失败提交。

## 端到端（E2E）与四条测试线矩阵

四条产品测试线统一收敛到 `cargo dev` 子命令，既可在本机运行，也可直接映射为 GitHub Actions 标准无显卡云 Runner Job：

| 测试线 | 单元 / 集成（L1） | 端到端 E2E（L2） | 云 Runner 图形/运行方式 |
| --- | --- | --- | --- |
| **Dedicated Server** | `bbb-protocol`、`bbb-registry-codec`、`bbb-server-*`、`bbb-admin` | `cargo dev e2e server`（`scripts/server_playable_gate.py`） | 纯无头 TCP + MCP + `bbb-client --probe-server`，无需显卡或 Vanilla 外部缓存即可跑 `bbb` 模式 |
| **Web Client + Gateway** | `bbb-web-host`（HTTP/COOP/COEP、WS 桥、资源构建）+ `wasm32` 编译检查 | `cargo dev e2e web --software-gpu`（`scripts/web_playable_gate.py`） | Headless Chromium（CDP + COOP/COEP + SharedArrayBuffer + SwiftShader WebGPU + `/mcp` + 异步截图） |
| **Linux Desktop Client** | `bbb-client*`、`bbb-render-types`、`bbb-mcp` | `cargo dev e2e client --software-gpu`（`scripts/mcp_playable_gate.py`） | Headless Weston Wayland + Mesa Lavapipe Vulkan（`BBB_SOFTWARE_GPU=1`）+ OpenAL `null` 驱动 |
| **Windows Desktop Client** | `bbb-client*`（含 Win32 平台、图标、MP4 编码单测） | `cargo dev e2e client --software-gpu`（`scripts/mcp_playable_gate.py`） | 原生 Win32 窗口 + Direct3D 12 WARP 软光栅适配器（`BBB_SOFTWARE_GPU=1`）+ OpenAL `null` 驱动 |

## Live 与发布级验收

以下真实进程、图形会话与发布压力门禁按改动表面或发布节点运行：

- **客户端确定性回放**：`scripts/client_replay.py` 是客户端重构的等价判据。`record` 经 `bbb-server` 录制场景，
  把 clientbound 帧按 tick 窗口存进缓存目录 `client-replay/<场景>/capture`；`replay` 由假服务器
  按同样窗口喂给 `--lockstep` 客户端（时钟只随 `sim.step` 前进），输出 serverbound 包、截图、
  可听音效与本地文件；`compare` 逐项比对，截图走 `bbb-replay-diff` 并写差异图。Linux 下自动启动
  headless Weston，Windows 下直接使用原生窗口。
- **桌面客户端 MCP 游玩与回放 E2E**：`cargo dev e2e client`（底层 `scripts/mcp_playable_gate.py`），
  通过真实 MCP 验证输入 schema、移动/挖放/生存背包拖拽的客户端事实与服务端结果、截图失败恢复和图片返回，
  再用同一输入计划完成录制及两轮 exact 回放比对。
- **多场景自主 MCP 游玩验证**：`cargo dev python scripts/mcp_agent_scenarios.py --mode policy --client <native|web> [--software-gpu]`
  覆盖 S1–S6 六个零作弊生存闭环场景（服务端 MCP 仅负责建场与独立裁决，游玩全程仅调用客户端 `/mcp`）；
  `--mode serve --ready-file <path> --done-file <path>` 仅发布单场景客户端 `/mcp` 与任务描述供外部 LLM 盲测。
- **Web 客户端可玩 E2E**：`cargo dev e2e web`（底层 `scripts/web_playable_gate.py`），通过真实 `bbb-server` +
  `bbb-web-host` + Chromium WebGPU 验证进服、Rayon Worker 区块网格上传、移动、MCP 与画布截图。
  默认调用 Cargo 增量构建并重新打包当前工作树的 WASM，同时构建两个原生程序。指定 `--skip-build` 可复用已有产物，
  此时用 `--wasm-dir` 或 `BBB_WEB_OUT_DIR` 选择 bundle。`--artifact-dir <dir>` 保留日志、截图与
  `web-playable-summary.json`。摘要记录 Git HEAD、工作树脏状态、实际 HTTP 响应解压后的 WASM SHA-256、
  Chrome DevTools Protocol 报告的浏览器版本与客户端渲染器日志中的适配器信息。
  性能摘要包含导航到门禁首次观测到可玩地形的时间（含资源包同意和轮询）、进入该状态后到截图的 rAF 间隔及
  WASM 线性内存水位。每轮使用全新的浏览器配置目录和世界，固定种子；导航前的大小探测会预热网关资源缓存。
  比较时使用相同机器、浏览器版本、GPU 模式和种子。需要限制回退时，按实测基线显式传入
  `--max-first-playable-ms`、`--max-wasm-mib` 或 `--max-frame-p95-ms`。
- **服务端可玩 E2E**：`cargo dev e2e server`（底层 `scripts/server_playable_gate.py run bbb`）用
  `bbb-client --probe-server` ↔ `bbb-server` 跑 flat 场景与 operator 会话，拒绝 unsupported packet、
  world apply error 与未完成的 chat/break/place/inventory/ack。
- **服务端发布族验收**：`cargo dev e2e server-release [FAMILY ...]`（底层 `scripts/server_release_gate.py`）的
  concurrency、chunk_load、restart、hostile、soak、signal、creation、autosave 八族由真实
  `bbb-server`、`bbb-client`、`bbb-admin` 驱动；tick health 以 100-tick 窗口平均不超过 50ms、
  循环不落后 1s 为硬界限。chunk_load 族在全新普通世界里让 20 个客户端同时进服并 forceload 256 个区块，
  要求单 tick 不超过 1s、tick p99 低于 50ms，用 release 版 `bbb-server` 跑。采样从客户端启动前
  到函数在单 tick 检查全部 forced chunks 已加载并读回结果为止，不混入等待客户端结束的空闲 tick。
  要复测分散探索，运行 `cargo dev e2e server-release chunk_load --disperse-players`。该场景把 20 名玩家
  送到独立的新地形，要求 256 个强制区块和 20 个落点在同一 tick 全部就绪，结果记录在 `chunk_load/burst.json`。
  收据同时记录各探针从开始连接到解码首个地形列的耗时范围和中位数。这段延迟包含登录和配置阶段。
  `cargo dev test --release -p bbb-server measure_cold_respawn_to_terrain -- --ignored --nocapture`
  单独测量重生到未加载区块时的重生包和目标地形到达时间，使用固定种子及真实 TCP 会话。
  harness 通过 `BBB_SERVER_TICK_SAMPLES=1` 启用逐 tick 日志；默认关闭，不保留额外采样历史。
  creation 族验收 BBB 创建、在线保存、重启恢复方块状态及再次可玩，不要求官方服务器读取 BBB 写出的存档。
- **服务端性能样本**：`scripts/server_bench.py` 的 20 客户端负载须全部进入 Play 并完成 hold；
  最后一名加入后跳过当时的不完整窗口，再固定采 3 个完整 100-tick 窗口，期间保持 20 人连接。
  不满足负载条件的运行不能用于性能结论。
- **Linux 桌面 compositor / IME / 长稳验收**：`scripts/playable-gate.sh`、`scripts/lifecycle-soak-gate.sh`、
  `scripts/platform-acceptance-gate.sh`（POSIX 侧交付取 Linux x86_64 / Wayland / Vulkan 行；
  Windows x86_64 / DX12 原生主机验收见[Windows 客户端](windows-client.md)）。

## 发布

决定发布时：关闭本次范围内的 ledger 功能项 → clean HEAD 上跑 `cargo dev gate` → 跑适用 live 验收 →
用 annotated tag 标记该 commit。tag 只标识通过验收的 commit，不证明后续 HEAD。

桌面端（Windows `x86_64-pc-windows-msvc`、Linux `x86_64-unknown-linux-gnu`）与服务端/Web 网关（Linux `x86_64-unknown-linux-gnu`、macOS `aarch64-apple-darwin`）的发布打包配方与桌面启动器见 [`packaging/README.md`](../../../packaging/README.md)；推送 `v*` tag 或手动触发 CI 的 `build_release_artifacts` 输入会在 [`.github/workflows/ci.yml`](../../../.github/workflows/ci.yml) 中构建并上传对应发布归档。

## 调试工具

官方 26.1 differential / world-oracle harness 只用于定位行为差异，不是验收或关闭条件，也不
阻塞提交。仓库 owner 已接受 EULA 并授权运行；仅在单次命令环境设
`BBB_PLAYABLE_ACCEPT_EULA=1` 或 `BBB_OFFICIAL_CAPTURE_ACCEPT_EULA=1`，不写入仓库或用户配置。

- `cargo dev e2e server live`（`scripts/server_playable_gate.py run live`）：同一 flat replay 对比官方服与 BBB 的归一化语义。
- `scripts/server-world-oracle-26.1.sh`：官方服与 BBB 在同一存档上 step 相同 tick 后比较方块/实体。
- `scripts/server-round-trip-gate-26.1.sh`、`scripts/server-dimension-runtime-gate-26.1.sh live`：
  官方存档读写往返与跨维度传送/存档对照。
- `scripts/capture-official-world-26.1.sh`：生成官方存档，用于验证 BBB 读取官方存档。
- `scripts/generate-noise-overworld-light-fixture.py --check`：固定方块窗口的光照与官方对照，只作调试。
  不改 worldgen 的重构步骤保留修改前后生成产物逐字节比对；worldgen 自身改动以同输入确定性和
  群系/结构/矿物存在性等可执行结果验证，不以官方逐方块一致或内部 RNG 调用序列判成败。

这些工具只把结果写到各自的 artifact 目录与标准输出，不写入仓库。

probe 差分的已知陷阱：

- 官方服的 chat 命令在 tick 间隙执行，chat fence 不是屏障，`/tick step` 是赋值不是累加；需要
  屏障时用 packet phase 答复的 completion request。
- 官方服只在接受移动包或实体换 section 时把已 tracked 实体配对给玩家；不动的 probe 看不到
  join 前已存在的实体。差分红了先单跑官方半场。
- probe 不跑客户端 tick，实体位置要读插值目标值而非 `EntityState.position`。

## 客户端 live 陷阱

- seat 输入走常驻 `uinput` 设备（`scripts/seat_device.py`），不用 virtual-keyboard 协议；compositor
  按键重复必须关（`repeat_rate 0`）；后 map 的客户端会夺焦点，失焦的 Play 会自我暂停，只有
  Escape 能关暂停菜单。
- 固定窗口尺寸先去 decoration、切 floating，等 configure 稳定后再 resize。
- IME 验收在 fcitx5 上打 `nihao`，以非 ASCII 的 chat draft 为判据（`ActiveByDefault=True`）。
- lifecycle soak 的趋势判据是「post-warmup 严格单调上升且 spread 超限」，iteration 取 5，
  RSS spread 限额 256 MiB（噪声带约 200 MB）；FD/thread 是精确探针。
- POSIX live 门禁的进程监管走 `scripts/process`（独立 process group、monotonic deadline、TERM→KILL）。

## 中断与接手

接手先看 `git status --short`、`git worktree list` 与 `cargo worker status`，以实际
diff 为准。保留其它 session 的 dirty/ahead worktree、分支与租约；不为得到 clean 状态 reset、
stash 或 force-remove。只清理本次创建且已集成或明确放弃的 worktree。

## 性能回归对照

客户端性能回归对照使用 Java 26.1 Fabulous 预设（仅按场景覆盖渲染视距，32 区块为主视距；Windows 使用原生 DX12 Release），
在同一主机运行同版本资源、同一服务端世界、相同身份、视角、分辨率和等价设置，关闭 VSync、帧率上限和录像，
区分已加载稳态、区块加载及移动场景，不通过降低有效视距、画质、实体或粒子负载达标：

| ID | 场景与验收面 |
| --- | --- |
| P01 | 普通自然地形静止：8、16、32 区块有效视距 |
| P02 | 同一自然地形连续转头：8、16、32 区块视距 |
| P03 | 移动与区块加载：预生成地形中的实际移动、首次进入与再次加载 |
| P04 | 全暗与火把洞穴：8、32 区块视距，保留正常取光 |
| P05 | 多层玻璃及转头：16、32 区块视距，保持透明排序 |
| P06 | 水下：8、32 区块视距，保留正常水体遮挡与雾 |
| P07 | 雨天：16、32 区块视距，确认实际降雨及相同天气负载 |
| P08 | 密集实体：16、32 区块视距，相同种类、数量、位置与状态 |
| P09 | 持续粒子：16、32 区块视距，相同事件和发送负载 |

## 文档测试

`cargo dev docs-lint`（`scripts/docs_lint.py`）是跨平台文档与仓库不变量检查入口；无网络运行，覆盖断链、
重复 mechanism id、孤儿页、ledger 条目规范、禁止的历史文档布局、快速离线生成事实新鲜度与单文件行数棘轮。
传入 `--all-generators`（`cargo dev gate` 默认启用）会额外对慢速离线 Vanilla 源码生成器及全部 19 个
基于 Vanilla JVM 运行时的 `generate-*.py` 生成器执行 `--check` 新鲜度校验。
不通过排除目录或放宽规则绕过失败。

## 日常开发与发布

日常开发不区分阶段，HEAD 可以领先 `last-gated`，状态即 `DRAFT/UNGATED`，不声称已发布。
发布按[发布](#发布)执行。
