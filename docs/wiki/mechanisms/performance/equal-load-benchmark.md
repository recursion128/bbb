# 官方 Vanilla 26.1 等负载性能比较

机制 ID：`performance-official-equal-load`

## 机制目的和边界

该机制在同机、同 compositor、同服务端权威相机序列和渲染一致性门禁下，比较未修改
官方客户端与 BBB。它提供 end-to-end frame interval、进程 CPU 均值，以及经内核提交链
关联出的 per-client GPU execution duration；present interval 与 GPU telemetry 仍是各自
独立的量，不得互相冒充。

## Vanilla 26.1 权威实现

离线启动证据见[offline runtime](../../architecture/offline-runtime.md)。相机路径由
`TeleportCommand.performTeleport` → `ServerPlayer.teleport` →
`ServerGamePacketListenerImpl.teleport` → `ClientboundPlayerPositionPacket` 生成，
`ClientPacketListener.handleMovePlayer` 消费。官方 jar 不 patch、不加 mod/agent。

## 决策输入与状态所有权

version/assets/world/resource/replay/settings/scene hash 定义运行身份。server tick 拥有相机
pose；客户端只接收 packet。compositor、GPU、kernel 和工具版本属于 host evidence；
每次 run 的 raw artifact/summary 属于外部目录，仓库只拥有 schema、generator、parser
和合同。

## 判断与处理顺序

1. 校验官方 jar、libraries/natives/assets，生成固定 options 和独立 gameDir。
2. 从只读 source world 创建 run copy，加载固定 datapack/replay。
3. warm-up 后按 marker capture 三场景：`static_orbit`、`preloaded_flight`、
   `chunk_boundary_crossing`。
4. 同 tick 检查 pose、camera、settings、chunk/section/entity/particle 与 material ROI。
5. 视觉/workload/draw 通过后生成 `evidence-sweep` acceptance，提交仅含 closure metadata、
   且 semantic-contract hash 不变的 sealing commit 并 full gate；每次 timing smoke 都用
   `--consistency-receipt` 指向
   `docs/evidence/vanilla-26.1/parity-acceptance.json`，冻结该 commit/hash。
6. 只有每个 run 都携带同一 pre-capture receipt，才按 Java → BBB → BBB → Java 的 ABBA 轮次采样。
7. parser 拒绝 hash/correction/resource/resolution/content/throttle/hook/receipt 异常，再聚合 raw。
8. 稳定性先看绝对量：application frame 的 mean FPS/frame、p50/p95/p99、1% low、
   stddev、p99-p50、hitch，以及 GPU duration 的 mean、p50/p95/p99、stddev、p99-p50、
   hitch。CV 只诊断；绝对指标全过时，CV 单独较差不产生 blocker。
9. summary 同时检查 combined、scene、ABBA round 与预声明 nearest-run pair；后续 BBB
   candidate 还要先完成交错 BBB-baseline A/B，FPS margin 由至少四次 baseline 重复噪声
   预先派生并封顶 1%。

macOS schema-3 BBB profiler 另输出 `frame_phases.csv`，将 encode-before-wait、
previous-submission wait、surface acquire、presentation tail、queue submit、drawable
present、async GPU poll 与完整 render call 从旧 aggregate `frame/render` 中拆开。Retina
validator 从 hashed archive 与 raw sidecar 重新派生各阶段 p50/p95/p99/stddev。由于
render-start interval 描述当前工作之前刚结束的区间，归因相关性按 phase[N] 对下一条
interval[N+1] 计算，并将同-row prior correlation 单列；官方 archive 不伪造 BBB 内部阶段。
阶段数据只作调度归因，不能单独成为性能胜负结论。
当前 frame-phase schema v2 进一步把 macOS Immediate 上一 submission 的 non-blocking
completion poll 与 poll 后仍 busy 才发生的 blocking fence wait 分列；逐 tick
`poll + blocking <= total previous-submission synchronization`，否则 archive 在聚合前拒绝。
该 schema 不授权 completion-poll 策略：full-gated `36bd347d7` 的接电 paired A/B 仅在
110/17,252 候选帧跳过 blocking wait，并产生 49 条 FPS/绝对稳定性 failure，已由
`completion-poll-ac-candidate.json` 在 Vanilla ABBA 前拒绝。恢复后的固定 budget-one
策略将 poll 列写零，继续保留 blocking/total 诊断与旧 Linux/report 兼容。
当前 repo evidence `docs/evidence/macos-native-retina/frame-phase-diagnostic.json` 来自
full-gated `f0cc03ad7` 的 BBB-only 三场景 probe，并由 full-gated `c7aa715af` 按 lag-1
口径复算；它触发的等价 flight-controller 重构已在 full-gated `9802b1ef5` 完成，仍保持
单 in-flight 与 wait→acquire→submit→present。该 probe 不是正式 BBB/Vanilla 性能比较，
也不替代后续独立策略的 paired BBB A/B 或 Java→BBB→BBB→Java ABBA。

候选 A/B 使用独立 `macos-native-retina-bbb-ab-v1` report schema。`candidate-check`
先拒绝 2560x1600 drawable、fullscreen、VSync/cap、场景、workload/draw 和 input
parity 不匹配；只有 parity 通过才比较性能。FPS 与 App/GPU 绝对稳定性都必须包含
combined、三个 scene、两个 ABBA round，以及四个 nearest pair 的 all-scenes 和逐 scene
scope。App
mean 只使用预声明 FPS margin，p50/p95/p99/stddev/p99-p50/1% low/hitch 不借用该
margin；GPU mean/p50/p95/p99/stddev/p99-p50/hitch 同样逐 scope 非回退。CV 永远保留
为诊断字段，绝对指标全部通过时不单独阻止；缺任一 scope 或任一 pair 失败均在正式
Vanilla ABBA 前拒绝。四次 baseline 重复噪声超过 1% 时必须重采，不能截断成 1% 后
继续验收。可用
`scripts/macos_native_retina.py candidate-check --report <report> --candidate-commit <sha>`
独立复核。

新的低延迟 surface candidate 使用向后兼容的 `macos-native-retina-bbb-ab-v2`：v1 历史
证据继续接受原字段，v2 额外强制 macOS Immediate、baseline/candidate
`desired_maximum_frame_latency=2/1`、wgpu Metal 预期 drawable count 3/2、两侧显式
submission budget 均为 1、未增加 queue depth 且 lifecycle 顺序不变。运行日志缺失
surface presentation 字段时 fail-closed；validator 还按 B-C-C-B×2 顺序逐 run 核对实际
观测到的 2/1 hint 与 3/2 预期 drawable count，不能只信顶层声明。这组字段只证明调度
合同，不能替代 22-scope FPS/绝对稳定性比较。

full-gated `969ea38f7` 的正式接电 B-C-C-B×2 正好验证了该 fail-closed 路径：8 个 run
均为 fullscreen 2560x1600，逐 run hint 2/1 与预期 drawable count 3/2 全部一致，但
validator 拒绝全部 22 个 FPS scope 和 44 个绝对稳定性比较。combined FPS
153.996→142.297，App p99 10.135→15.927 ms，GPU p99 0.877→0.893 ms；0 个
>=33.333 ms hitch 不能抵消其它绝对指标回退。报告为
`docs/evidence/macos-native-retina/low-latency-surface-ac-candidate.json`（SHA-256
`662d716afdf23442713dbab6ed70892948db8246949c00b36ddaaabe9c08a939`）。默认 hint 2
已恢复，该候选不进入 Vanilla ABBA。

`macos-native-retina-bbb-ab-v3` 及其 0.19→wgpu 30 失败报告只作历史迁移证据，不再参与
候选判定。active v4 baseline 是 full-gated `5b1903221` / release SHA-256
`849a9300ed1b1325137506f7519782b57c264eb31347d36725c96686afa20817`；四次 AC repeat
195.828/194.414/194.422/194.180 FPS 派生 0.8465% noise margin。B-C-C-B×2 的两侧均须为
registry wgpu/core/hal/types/naga 30.0.0，并逐 run 绑定 dependency receipt、binary hash、
`PassBoundary` timestamp strategy、Immediate/hint 2、实际三个 drawable、单 in-flight、
2560x1600 fullscreen 与相同 workload。任何 identity 或 measurement-scope mismatch 都在
FPS/稳定性比较前 fail-closed；combined、三个 scene、两个 round、全部 nearest pair 与逐
scene pair 的 FPS/绝对 App/GPU scope 全过后，才允许正式 Java ABBA。

## 生命周期和 fallback

所有 client/server/compositor/tool 进程进入独立 process group 并有 bounded cleanup。
MangoHud 对任一 API hook 失败或自身 median overhead >1% 时不作正式结果。GPU 采集
的 association ratio 不足或 overhead 超同一阈值时，只报告共同 frame interval 与 GPU
load，不降格猜测。

## BBB 实现映射

`scripts/official_client_benchmark.py` 拥有 audit、launch、replay generation、smoke、
capture parsing 和 summary；`scripts/fixtures/official-client-benchmark/replay-26.1.json`
是 schema-1 replay。BBB 运行时只提供只读 pose、visible/resident 和 draw counters，不
用 `bbb-control` 驱动相机。

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 官方离线真实启动 | `Main`/`Minecraft` offline branches | benchmark launcher | `equivalent` | official smoke |
| 共同相机 replay | player-position packet path | replay/datapack + world apply | `equivalent` | camera hash/marker logs |
| 渲染/可见 workload gate | official screenshot/draw | BBB counters + ROI/draw evidence | `equivalent` | 3 scene anchors |
| frame interval | MangoHud OpenGL/Vulkan hook | parser/summary | `equivalent` | overhead audit + ABBA raw |
| CPU process means | `perf stat` client PID | normalization parser | `equivalent` | event availability + raw CSV |
| per-client GPU duration | amdgpu 提交链（radeonsi/RADV 共用） | `scripts/gpu_duration_capture.py` | `equivalent` | 双 API 关联率 1.0 + overhead 审计 |

## 结构性不变量

- view/simulation/entity distance、graphics/cloud/particle/mipmap/HUD/FOV/resolution 相同。
- 不得降低 BBB resident cap、实体数、透明面或粒子获得优势。
- draw call 数只证明非空内容；API batching 不同，不能要求数量相等或用作性能倍数。
- 每客户端至少四次有效 capture；正式流程采用两轮完整 ABBA 并保留 machine-readable raw。
- implementation/checkpoint full gate 不自动启动 release、视觉、RenderDoc 或 ABBA；当前 stage
  只约束新采集，不作废已有 pre-capture receipt 证明且 planner 判定未受影响的 formal artifact。
- code freeze 后 owner/path、focused contract、capture schema/settings/input 或 binary/load image
  变化时，只失效 acceptance planner 指出的机制和 validation class；最终仍在 clean gated HEAD
  运行 strict acceptance；`final-performance` 先拒绝 scoped consistency receipt，`final` 再拒绝
  任何 formal artifact 的 scoped reuse，因而旧 domain sweep 不能解锁新 ABBA。
- 每项性能优化都重跑 planner 判定受影响的视觉锚点和 workload gate。
- sleep、固定 pacing、FPS cap、VSync 或减少 visible/resident/draw workload 不能作为稳定性
  优化；candidate 即使仍快于 Vanilla，只要超过固定 BBB baseline 的 FPS margin 仍拒绝。

## 聚焦测试（focused tests）和真实证据

`scripts.tests.test_official_client_benchmark` 覆盖 metadata/classpath/native、hash、offline
CLI、replay schema/顺序/hash、pose 序列、MangoHud/perf/ftrace parser、percentile、1%
low、hitch、per-frame normalization、invalid reason 与 cleanup；
`scripts.tests.test_macos_native_retina` 另覆盖 panel-native/drawable 先行拒绝、raw/archive
互证、stddev 与 p99-p50 派生、CV-only 非阻塞、绝对回退、逐 run scope 及 BBB baseline
FPS 非劣，以及 frame-phase 列/配对 tick/sidecar/schema 拒绝与阶段统计相关性。

### 当前 `d2f210805` exact-HEAD 一致性 sweep

当前性能前置证据全部来自 clean/full-gated
`d2f21080548603a9d3bd9df6f9c07e8c23d6c49d`，BBB release binary SHA-256 为
`985540ca2298e26209d336e34483b0a8301d8bfe26215ffa9384daabcd535e9f`。
Vanilla→Vanilla→BBB→BBB 的三个共同 replay 重复 capture 保持 637 resident/selected chunks、
15,288 resident/selected/uploaded sections，以及相同 replay/world/resources/settings/scene/camera
hash。冻结 ROI、结构断言和六份 RenderDoc capture 均通过。机器摘要为：

- lifecycle：`/var/tmp/bbb-domain-d2f210805/lifecycle/report.json`，SHA-256
  `2e2606d3ecf33e60c9e51edc87353ebcfe96d38de28e22f8c446fcdb2f3df9a2`；
- Linux platform：`/var/tmp/bbb-domain-d2f210805/platform/report.json`，SHA-256
  `8b67a5eb8d2ed6326ca0bbbcc83a4a2bdea7653ed3dd77a6866f681100ba37a5`；
- visual/workload：`/var/tmp/bbb-parity-d2f210805-final/visual/workload-summary.json`，SHA-256
  `f48e8b2f6fae15211cb1d023020bad0aa94f4f42fe9272f7e35025a817fab0b3`；
- draw：`/var/tmp/bbb-parity-d2f210805-final/renderdoc/summary.json`，SHA-256
  `5b4428a32833e6a52c6dd83fadda509b790c1837f4a6be88da1666b69e712813`。

这些 receipt 签发 full-gated `a0781a7eb613e1dd08c1db4fc5d3770adf9105f1`
evidence-sweep consistency receipt；随后完成的两轮 Java→BBB→BBB→Java 与两侧 exact GPU
trace 汇总位于 `/var/tmp/bbb-parity-d2f210805-final/final-performance/formal-summary.json`，
SHA-256 为 `c5930d19a414fad69cc1f402d5a8a3bd5ffc5094c732d643f06c74f0042bcb54`。

当前 MangoHud application-frame 的 BBB/Vanilla p50 为 `2.737/3.356 ms`，p95 为
`4.225/5.011 ms`，p99 为 `4.703/5.791 ms`，两侧均无正式帧 ≥33.333 ms hitch。
按 presented frame 归一的 BBB task-clock/cycles/instructions 分别比 Vanilla 低
`5.109%/0.693%/2.430%`。GPU submit→dispatch→finished-fence association 两侧均为 `1.0`；
GPU execution p50 为 `1.505/2.656 ms`（Vanilla/BBB），p95 为 `3.192/3.004 ms`，p99 为
`4.231/4.420 ms`，作为独立 GPU job 指标如实报告，不与 application-frame duration 混写。
MangoHud 与 GPU trace 对两侧 Weston surface-commit p50 的影响均低于 1%。

### 历史 `675cbfc53` 性能回归基线（不关闭当前行）

sealed/full-gated receipt `6e2d23e7f` 之后已完成两轮 Java→BBB→BBB→Java 和两侧 exact GPU
trace；formal summary 位于
`/tmp/bbb-parity-675cbfc53-final/final-performance/formal-summary.json`，SHA-256 为
`5fa8dbb86169f1c117137df506892e3f444550245393a5393fe3c721226c3706`。汇总器重验八次
raw 的 receipt、binary、camera/world/resource/settings/scene hash、零位置修正、MangoHud/perf
与 workload/draw linkage，`formal_comparison_complete=true`、`metrics_publishable=true`。

| 当前正式指标 | Vanilla | BBB |
| --- | ---: | ---: |
| application-frame p50 / p95 / p99 | 3.437 / 5.083 / 5.838 ms | 2.817 / 4.294 / 4.764 ms |
| >=33.333 ms hitch | 0 | 0 |
| task-clock / presented frame | 1.080 ms | 1.045 ms |
| cycles / presented frame | 4,449,322 | 4,531,010 |
| instructions / presented frame | 6,840,948 | 6,668,909 |
| GPU execution p50 / p95 / p99 | 1.538 / 3.272 / 4.296 ms | 2.757 / 3.114 / 4.495 ms |

GPU trace 的 Vanilla/BBB association ratio 均为 `1.0`（83,263/37,192 graphics jobs），
baseline→trace Weston p50 overhead 为 `+0.012%/-0.007%`；MangoHud overhead 为
`-0.028%/-0.031%`。负数只表示测量噪声内未见开销，不解释成性能收益。GPU execution
是独立 submit→scheduler→finished-fence 指标，不能替代 application-frame 或 CPU 指标。

### 上一轮历史结果（不可关闭当前性能行）

本轮正式 capture binary 属于 commit
`20bcefd4916f4c1e3a11934b2c41ba09d7b5e61c`，SHA-256 为
`35685227c3e49454c4b6d0deee5a4503a4c4bfc051932a4c9fdeda69690a9f3d`；独立 release
rebuild 属于 full-gated evidence commit
`dd5890673b4cb979b1244ecf85f37cd516c79da6`，SHA-256 为
`bdfbf2bf608adacfa12b742a85470b6f00c0c48dc08757e669c46d780671a768`。两者的
`elf-load-image-v1` 均为
`c5ffeef9796699357a13ddff12cebee6e87f1e772baaa43aadeaab77b34d6d64`；机器报告保留
两份文件、raw hash、entry、program headers、`PT_LOAD` 与非 build-id notes，只有该
执行映像 identity 完全相同才允许绑定。八次 timing run 还统一绑定 full-gated
`f01ed702e51d9753914ea1d3ae6836a1f76e5e1b` 的 evidence-sweep receipt。

一致性前置门禁已按 Vanilla→Vanilla→BBB→BBB 顺序重采：三场景双侧重复
噪声标定后冻结既有 ROI 阈值，637 resident/selected chunks、15,288 resident/selected/
uploaded sections、结构审查和六份 OpenGL/Vulkan RenderDoc capture 全部通过。workload
summary 位于 `/tmp/bbb-parity-dd5890673-final/visual/workload-summary.json`，SHA-256 为
`d07e190f0dda635735d78f988900cfff38e4313b52b068a38d37677b1be51368`；draw summary 位于
`/tmp/bbb-parity-dd5890673-final/renderdoc/summary.json`，SHA-256 为
`e2ea4805a8892656fb1fda16d1d5af16880301b67e6a5a6abe84bd45beb86f37`。正式性能摘要位于
`/tmp/bbb-parity-f01ed702e-final/formal-summary.json`，SHA-256 为
`1a2f38d7c8a6b6fc17f92fa814dd338c045e772bf880e2e329a62b69c123497e`。

失败证据未删除：本轮第一次 ABBA 使用 evidence rebuild 的原始文件 SHA，虽与视觉 capture
具有相同 load image，但不满足正式汇总器要求的共同 capture 文件 identity；八次 raw 保留于
`/tmp/bbb-parity-f01ed702e-final/formal-abba`，机器失败 manifest/stderr 精确记录
`BBB captured binary does not match the capture receipt`，随后用 provenance 实际绑定的冻结
binary 完整重跑两轮 ABBA。更早的 RenderDoc 单进程崩溃和错误包含 compositor 的 GPU summary
也继续保留；正式 summary 只接受 smoke 记录的精确 client PID。

| Contract/evidence | SHA-256 |
| --- | --- |
| replay | `5073908b5e5cd7eeca023540b9c2844e9c4d196610a2fbb67866058c5421c388` |
| world snapshot | `6ab2ef99bd6c816b68e54608142206f38aa6f3644f087c477d2367ce50173fd1` |
| resources | `8aa8d02d6806ef9396564b83b7b64f73014ea654d8059a1b8ae6b09f3f67df0c` |
| settings | `5d67fe2070707f14dbc3420313e9af7ef239914dd3daa50356c05a02609da1cd` |
| scene | `d3fdd2005e6a03d6531aa82b2044472ac8c74dbc28f276b8e2bfd5ca9d63aad2` |
| server camera log | `f3749c011859c34d97f5ce0f36fdfec45e2ab7c81b51b405ad046489943a0b3a` |
| Vanilla self-calibration | `a532e15a09da8bdc2cd228e563a55c7cd8cdf1d7b211befb12eb821a9783889f` |
| BBB self-calibration | `c09c228781068956365226151f675a4940d6a3e66ecc447471e073f4926e1501` |
| visual/workload gate | `d07e190f0dda635735d78f988900cfff38e4313b52b068a38d37677b1be51368` |
| RenderDoc anchor summary | `e2ea4805a8892656fb1fda16d1d5af16880301b67e6a5a6abe84bd45beb86f37` |
| formal ABBA summary | `1a2f38d7c8a6b6fc17f92fa814dd338c045e772bf880e2e329a62b69c123497e` |

MangoHud application-frame 结果如下；1% low 来自尾部分布，不能从 p99 直接推导：

| 场景/客户端 | Samples | p50 ms | p95 ms | p99 ms | 1% low FPS |
| --- | ---: | ---: | ---: | ---: | ---: |
| overall Vanilla | 57,080 | 3.436 | 5.084 | 5.851 | 163.30 |
| overall BBB | 70,617 | 2.819 | 4.298 | 4.760 | 171.58 |
| static Vanilla / BBB | 17,530 / 22,109 | 3.849 / 2.972 | 4.185 / 3.273 | 4.381 / 3.405 | 204.47 / 222.82 |
| flight Vanilla / BBB | 21,930 / 26,867 | 3.038 / 2.498 | 3.540 / 2.785 | 3.873 / 2.974 | 241.58 / 206.40 |
| boundary Vanilla / BBB | 17,620 / 21,641 | 3.490 / 2.845 | 5.739 / 4.689 | 6.085 / 4.964 | 158.89 / 161.04 |

`perf stat` 的总体进程均值为 Vanilla/BBB task-clock
`1.042/1.045 ms/frame`、cycles `4,287,670/4,517,034`、instructions
`6,544,060/6,697,722`；这两种工具口径不能相减为 CPU 或 GPU 时间。MangoHud 0.8.4
archive SHA-256 是
`aa0b2a009552f5e7c50baabf361267a79ea5e5c0fea440786f3b202d51f57753`；RenderDoc
1.45 commit 是 `2fc0bc04cb95499635f63986a55bc6f67849dd9f`，archive SHA-256 是
`b0a7ee8ec78c4fa511eb44137380d99a748472e5fd24c877f8afcc860a172a42`。RenderDoc
只证明三个锚点的 OpenGL/Vulkan draw 内容非空；API batching 不同使 draw count 不能
作为等负载或性能倍数。

### CPU 优化与 2026-08-09 最终复测

`perf record` 将稳定场景的主要冗余定位为：淡入已经完成后仍逐帧遍历约 2,000 个可见
section 并重建相同的 renderer 映射；地形 cache drain 期间反复构造相同 selected/loaded/
required 集合；主线程单帧最多复制 8 个区块快照形成突发。当前实现以 world/visibility
revision 为失效边界复用前两类结果，并把快照抽取限制为每帧 1 个；最终目标仍为 637 个
区块、15,288 个 section，三个正式 capture start 都是 cache ready、pending jobs 为 0。

本轮正式两轮 ABBA 中，BBB 相对 Vanilla 的 task-clock 为 `+0.35%`，instructions 为
`+2.35%`；cycles 为 `+5.35%`，按 presented frame 如实报告但不冒充时间。BBB 的
application-frame p50/p95/p99 均更低，双方均无 `>=33.333 ms` hitch。由此当前数据不再
支持继续以“重复 CPU 工作导致 BBB 劣于 Vanilla”为前提优化；后续只有新 profile 或
一致性修复造成回退时才重开性能 slice。

## per-client GPU execution duration

该指标测在两个图形 API 之下：radeonsi（官方客户端的 OpenGL）与 RADV（BBB 的 Vulkan）
共用同一条 amdgpu 提交路径，因此一次采集覆盖两者，且无需给任一客户端插桩。

| 链段 | tracepoint | 携带的关联键 |
| --- | --- | --- |
| 提交 | `amdgpu:amdgpu_cs_ioctl` | 在客户端自身进程上下文触发，`pid` 即归属 |
| 派发 | `amdgpu:amdgpu_sched_run_job` | `sched_job` / `(context, seqno)` / ring |
| 完成 | `dma_fence:dma_fence_signaled` | `driver=drm_sched` + `(context, seqno)` |

`drm/scheduler` 为每个 entity 分配两个相邻 fence context：`scheduled` 在
`entity->fence_context`，`finished` 在 `+1`。amdgpu 追踪的是 finished fence，所以
作业自身的 `(context, seqno)` join 到完成信号，而 `(context-1, seqno)` 是它开始执行
的时刻——join 错一个会把派发延迟当成执行时长。由此：

    queue wait = run_job - cs_ioctl
    execution  = finished signal - run_job

compositor 按进程排除，不靠推断：Weston 从自己的 pid 提交，永远不会落进客户端的数字。
实测同一次采集里 Weston、Xwayland 与官方客户端三个进程被干净分离。

采集在 runner 完成两项标准授权后以**非特权**身份运行（`perf record`，见
`scripts/python python scripts/gpu_duration_capture.py preflight`）：tracefs 的 tracepoint id 与
`header_page`、`header_event`、`printk_formats`、`saved_cmdlines`、`saved_tgids` 元数据对 benchmark
组可读，以便 perf 同时记录事件和可解码的 `TRACING_DATA`；`kernel.perf_event_paranoid=-1`
则允许打开事件。preflight 必须逐项检查，不能只以 event id 可读判定 ready。

AMDGPU 在当前受测内核上存在两种等价打印格式：旧格式给出
`sched_job=..., context=..., seqno=...`，Linux 7.0 格式给出
`fence=<context>:<seqno>`。parser 同时接受两种格式，关联键仍是同一个
`(context, seqno)`；未知格式会导致 association ratio 不足并使正式汇总失败。

发布该指标的判据是采集自身的 association ratio（完成整条链的客户端提交占比），低于
门槛的采集按失败报告，不降格汇总。分母只计 `amdgpu_cs_ioctl` 提交：调度器还会跑无
客户端归属的内核内部作业（SDMA 上的 TTM 缓冲搬运），把它们计入会低估一个实际完整的
关联，故单独计数而非静默丢弃。

`fence:fence_signaled` 在上游随 fence API 更名为 `dma_fence:` 子系统，是同一个完成
事件；早期把它记为“内核缺少所需 tracepoint”的判断源于名称，而非能力缺失。

本轮重新采集的 Vanilla/BBB association ratio 均为 `1.0`；graphics-ring client jobs
分别为 `83,295/37,194`。GPU execution p50 为 `1.542/2.753 ms`，p95 为
`3.270/3.115 ms`，p99 为 `4.336/4.493 ms`。GPU trace 对 Weston surface-commit p50
的影响为 `+0.007%/+0.009%`，MangoHud 为 `-0.023%/-0.022%`，均在 1% 门槛内。

## 当前差异

当前声明范围内没有已知一致性或同载性能差异。exact `d2f210805` 的三场景
camera/world/workload、冻结 ROI、结构/draw、两轮 ABBA、MangoHud/perf 与 GPU-duration
association/overhead 均通过。frame interval、进程 CPU 均值与 GPU execution duration 是三个各自独立测得的量，
任何一个都不能由另一个推导或相减得出；draw count 因 API batching 不同，同样不能作为
等负载或性能倍数。

## 账本/goal 路由

等负载比较与 GPU duration 只在 code-freeze domain sweep、最终两轮 ABBA、provenance bridge 和
machine-readable strict acceptance 全部通过后关闭账本项。平台、lifecycle 与 audio 状态只取
[`unsupported-features.md`](../../../unsupported-features.md)，不得由本 receipt 推导。

## 精确重开条件

采集链、fence context 配对规则、ring 归类或 runner 授权方式变化时重开；association
ratio 跌破门槛或 overhead 超阈值同样重开。

## 相关页面和源码索引

- [性能与验证边界](../../architecture/performance-and-validation.md)
- [官方离线运行体系](../../architecture/offline-runtime.md)
- [方块渲染](../rendering/block-rendering.md)、[fluid rendering](../rendering/fluid-rendering.md)
