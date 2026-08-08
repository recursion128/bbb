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
5. 视觉/workload/draw 通过后生成 `evidence-sweep` acceptance，提交仅报告的 sealing commit
   并 full gate；每次 timing smoke 都用 `--consistency-receipt` 指向
   `docs/evidence/vanilla-26.1/parity-acceptance.json`，冻结该 commit/hash。
6. 只有每个 run 都携带同一 pre-capture receipt，才按 Java → BBB → BBB → Java 的 ABBA 轮次采样。
7. parser 拒绝 hash/correction/resource/resolution/content/throttle/hook/receipt 异常，再聚合 raw。

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
- implementation/checkpoint full gate 不自动启动 release、视觉、RenderDoc 或 ABBA；代码冻结后
  才集中采正式 evidence。
- code freeze 后 owner/path、focused contract、capture schema/settings/input 或 binary/load image
  变化时，只失效 acceptance planner 指出的机制和 validation class；最终仍在 clean gated HEAD
  运行 strict acceptance，不把 scoped reuse 当成交付证据。
- 每项性能优化都重跑 planner 判定受影响的视觉锚点和 workload gate。

## 聚焦测试（focused tests）和真实证据

`scripts.tests.test_official_client_benchmark` 覆盖 metadata/classpath/native、hash、offline
CLI、replay schema/顺序/hash、pose 序列、MangoHud/perf/ftrace parser、percentile、1%
low、hitch、per-frame normalization、invalid reason 与 cleanup。

以下 receipt 只保留回归背景和工具链输入，不冒充当前 matrix 的关闭证据。本轮 capture commit 是
`fa73aed19ad54b9f501c1a0ab55db52a5aebe132`；它已在采集前 full-gated，冻结 release
binary SHA-256 为 `6d7ce00a65368a97535ac9024dfbc22a4bca2fbc48978e92ff68491416e5b071`。
机器报告同时保留 capture binary 和本页 checkpoint 后的独立 evidence rebuild；只有原始
字节 SHA-256 相同或 `elf-load-image-v1` 对 ELF entry、program headers、`PT_LOAD` 与非
build-id notes 给出相同执行映像 identity，才允许把 capture 绑定到后继 evidence commit。

current-binary 一致性前置门禁已按 Vanilla→Vanilla→BBB→BBB 顺序重采：三场景双侧重复
噪声标定后冻结既有 ROI 阈值，637 resident/selected chunks、15,288 resident/selected/
uploaded sections、结构审查和六份 OpenGL/Vulkan RenderDoc capture 全部通过。workload
summary 位于 `/tmp/bbb-parity-fa73aed19/visual/workload-summary.json`，SHA-256 为
`d10fc4dd6f0dead8dffd522e7796dbab4be78821da9b47db7a35b2f4b7c77c75`；draw summary 位于
`/tmp/bbb-parity-fa73aed19/renderdoc/summary.json`，SHA-256 为
`61631db9f530fdfe618fb1cb267cb1d8813fe377cfb18050da30295b00d9e992`。正式性能摘要位于
`/tmp/bbb-parity-fa73aed19/formal-summary.json`，SHA-256 为
`5536b018f0f63c4ff48de05d7ae2f6b4e77d247458e51064ece909958acab9de`。

失败证据未删除：第一次把 Vanilla 三锚点串进同一 RenderDoc 进程时，第二次 control request
在 `pthread_setaffinity_np` 崩溃，原始目录保留于 `renderdoc/vanilla-x11`，随后按每锚点独立
进程 fix-forward；第一次 GPU summary 因空 client PID 过滤器错误包含 compositor，也以
`*-gpu-summary-invalid-all-pids.json` 保留，正式 summary 只接受 smoke 记录的精确 client PID。

| Contract/evidence | SHA-256 |
| --- | --- |
| replay | `5073908b5e5cd7eeca023540b9c2844e9c4d196610a2fbb67866058c5421c388` |
| world snapshot | `6ab2ef99bd6c816b68e54608142206f38aa6f3644f087c477d2367ce50173fd1` |
| resources | `8aa8d02d6806ef9396564b83b7b64f73014ea654d8059a1b8ae6b09f3f67df0c` |
| settings | `5d67fe2070707f14dbc3420313e9af7ef239914dd3daa50356c05a02609da1cd` |
| scene | `d3fdd2005e6a03d6531aa82b2044472ac8c74dbc28f276b8e2bfd5ca9d63aad2` |
| server camera log | `f3749c011859c34d97f5ce0f36fdfec45e2ab7c81b51b405ad046489943a0b3a` |
| Vanilla self-calibration | `dd71bbcde87b7a7ce12ee7b4878e389e64e1eab10e1e6f56c3dedd8a2b3b62ef` |
| BBB self-calibration | `d900d77c9ab60e37b999667c152cc51a9856bc505d5b1ca599d1b91e3c520097` |
| visual/workload gate | `d10fc4dd6f0dead8dffd522e7796dbab4be78821da9b47db7a35b2f4b7c77c75` |
| RenderDoc anchor summary | `61631db9f530fdfe618fb1cb267cb1d8813fe377cfb18050da30295b00d9e992` |
| formal ABBA summary | `5536b018f0f63c4ff48de05d7ae2f6b4e77d247458e51064ece909958acab9de` |

MangoHud application-frame 结果如下；1% low 来自尾部分布，不能从 p99 直接推导：

| 场景/客户端 | Samples | p50 ms | p95 ms | p99 ms | 1% low FPS |
| --- | ---: | ---: | ---: | ---: | ---: |
| overall Vanilla | 56,416 | 3.461 | 5.113 | 5.889 | 161.16 |
| overall BBB | 70,717 | 2.820 | 4.306 | 4.783 | 171.12 |
| static Vanilla / BBB | 16,981 / 22,265 | 3.939 / 2.970 | 4.201 / 3.287 | 4.374 / 3.417 | 184.59 / 224.64 |
| flight Vanilla / BBB | 21,832 / 26,942 | 3.047 / 2.498 | 3.548 / 2.803 | 3.881 / 3.023 | 238.88 / 205.42 |
| boundary Vanilla / BBB | 17,603 / 21,510 | 3.493 / 2.846 | 5.738 / 4.697 | 6.101 / 4.969 | 159.12 / 160.27 |

`perf stat` 的总体进程均值为 Vanilla/BBB task-clock
`1.067/1.043 ms/frame`、cycles `4,365,768/4,479,094`、instructions
`6,722,340/6,649,638`；这两种工具口径不能相减为 CPU 或 GPU 时间。MangoHud 0.8.4
archive SHA-256 是
`aa0b2a009552f5e7c50baabf361267a79ea5e5c0fea440786f3b202d51f57753`；RenderDoc
1.45 commit 是 `2fc0bc04cb95499635f63986a55bc6f67849dd9f`，archive SHA-256 是
`b0a7ee8ec78c4fa511eb44137380d99a748472e5fd24c877f8afcc860a172a42`。RenderDoc
只证明三个锚点的 OpenGL/Vulkan draw 内容非空；API batching 不同使 draw count 不能
作为等负载或性能倍数。

### 2026-08-07 CPU 优化复测

`perf record` 将稳定场景的主要冗余定位为：淡入已经完成后仍逐帧遍历约 2,000 个可见
section 并重建相同的 renderer 映射；地形 cache drain 期间反复构造相同 selected/loaded/
required 集合；主线程单帧最多复制 8 个区块快照形成突发。当前实现以 world/visibility
revision 为失效边界复用前两类结果，并把快照抽取限制为每帧 1 个；最终目标仍为 637 个
区块、15,288 个 section，三个正式 capture start 都是 cache ready、pending jobs 为 0。

本轮正式两轮 ABBA 中，BBB 相对 Vanilla 的 task-clock 为 `-2.24%`，instructions 为
`-1.08%`；cycles 为 `+2.60%`，按 presented frame 如实报告但不冒充时间。BBB 的
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
`scripts/python python scripts/gpu_duration_capture.py preflight`）：tracefs 对 benchmark 组可读以便 perf
解析 tracepoint id，以及 `kernel.perf_event_paranoid=-1` 以便打开它们。

发布该指标的判据是采集自身的 association ratio（完成整条链的客户端提交占比），低于
门槛的采集按失败报告，不降格汇总。分母只计 `amdgpu_cs_ioctl` 提交：调度器还会跑无
客户端归属的内核内部作业（SDMA 上的 TTM 缓冲搬运），把它们计入会低估一个实际完整的
关联，故单独计数而非静默丢弃。

`fence:fence_signaled` 在上游随 fence API 更名为 `dma_fence:` 子系统，是同一个完成
事件；早期把它记为“内核缺少所需 tracepoint”的判断源于名称，而非能力缺失。

本轮重新采集的 Vanilla/BBB association ratio 均为 `1.0`；graphics-ring client jobs
分别为 `82,928/36,040`。GPU execution p50 为 `1.549/2.758 ms`，p95 为
`3.277/3.123 ms`，p99 为 `4.334/4.491 ms`。GPU trace 对 Weston surface-commit p50
的影响为 `-0.024%/+0.021%`，MangoHud 为 `-0.062%/-0.033%`，均在 1% 门槛内。

## 当前差异

capture commit `fa73aed19` 的视觉/workload/draw、正式两轮 ABBA、MangoHud/perf、
GPU-duration association 与 overhead 曾全部通过，但当前 acceptance 把两条性能机制保持
`partial`：它们必须等 protocol/state/resource/rendering domain gate 在新的 code freeze 上通过，
再用最终 release 集中重跑两轮 ABBA。历史 receipt 可用于定位与阈值合同回归，不能越过阶段顺序
或冒充 final exact-HEAD。frame interval、
进程 CPU 均值与 GPU execution duration 是三个各自独立测得的量，
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
