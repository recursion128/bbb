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
5. 只有视觉/workload gate 通过，才按 Java → BBB → BBB → Java 的 ABBA 轮次采样。
6. parser 拒绝 hash/correction/resource/resolution/content/throttle/hook 异常，再聚合 raw。

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
- 至少三次有效 capture/客户端；正式流程采用完整 ABBA 并保留 machine-readable raw。
- 每项性能优化都重跑关键视觉锚点和 workload gate。

## 聚焦测试（focused tests）和真实证据

`scripts.tests.test_official_client_benchmark` 覆盖 metadata/classpath/native、hash、offline
CLI、replay schema/顺序/hash、pose 序列、MangoHud/perf/ftrace parser、percentile、1%
low、hitch、per-frame normalization、invalid reason 与 cleanup。

历史 receipt 只保留回归背景，不再冒充 current-binary 证据。configuration finish ordering
修复后的 capture commit 是 `6c16fda010ac5b5ec42396db00e24cdb5b22ceac`；其冻结 release
binary SHA-256 为 `93547b9b279edf1987f95ce8fb47921b46ea4d093ae971656de5820a6bfc2c0f`。
证据 commit `0bfd97ae0d7818e97fa8a097b61b22188f6fbbdf` 的独立 rebuild SHA-256 为
`2af84597c83cb5a0787308e565a4041202ba88600c3c63bab07d93a9b7a6630c`；两者原始字节不同，
差异只落在未加载 symbol/section string table 与 GNU build-id。`elf-load-image-v1` 独立解析
并锁定 ELF entry、全部 program headers、四个 `PT_LOAD` 段和非 build-id notes，得到相同
执行映像 identity `50a5881a70129e7abaa84225fdd882f2eeaf19f78e9fc8b6bc597b44bbad97a5`。
bridge 位于 `/tmp/bbb-parity-0bfd97ae0/provenance/6c16-to-0bfd.json`，SHA-256 为
`b8123a02b4c7c23dceec16c3444d4d9d054143079bd92121f1e4f1c5307c2e83`；当前 evidence
commit 已通过 full gate；本页的 current receipt 通过 checkpoint full gate 后，矩阵据此
关闭而不声称原始 ELF byte-identical。

current-binary 一致性前置门禁已按 Vanilla→Vanilla→BBB→BBB 顺序重采：三场景双侧重复
噪声标定后冻结既有 ROI 阈值，637 resident/selected chunks、15,288 resident/selected
sections、结构审查和六份 OpenGL/Vulkan RenderDoc capture 全部通过。经 bridge 重建的
workload summary 位于 `/tmp/bbb-parity-0bfd97ae0/visual/workload-summary.json`，SHA-256
为 `f48e81f3ce9f7a42cde3f55e97bf8b9ee4f3a20a418c6367f2bb8f61b92790f8`；draw summary
位于 `/tmp/bbb-parity-0bfd97ae0/renderdoc/summary.json`，SHA-256 为
`dc734dbc1f1290df20c4d9f5eef7c6133a320d8565883e88a53a6dfb976217b2`。正式性能摘要位于
`/tmp/bbb-parity-0bfd97ae0/formal-summary.json`，SHA-256 为
`cf591d2605b65478ee0239deb6ae489015cc8c9056cb23b6adf4d764d745ed01`。

| Contract/evidence | SHA-256 |
| --- | --- |
| replay | `5073908b5e5cd7eeca023540b9c2844e9c4d196610a2fbb67866058c5421c388` |
| source world | `59bc552530d1c13ce3d618be5669e5d99d052e01d53e99a879a44008c7ba4bca` |
| resources | `8aa8d02d6806ef9396564b83b7b64f73014ea654d8059a1b8ae6b09f3f67df0c` |
| settings | `5d67fe2070707f14dbc3420313e9af7ef239914dd3daa50356c05a02609da1cd` |
| scene | `6f77811b5deafacea3c7666414cc1705c396a987b015e2a5f1f4832e872f73bc` |
| server camera log | `f3749c011859c34d97f5ce0f36fdfec45e2ab7c81b51b405ad046489943a0b3a` |
| visual/workload gate | `f48e81f3ce9f7a42cde3f55e97bf8b9ee4f3a20a418c6367f2bb8f61b92790f8` |
| RenderDoc anchor summary | `dc734dbc1f1290df20c4d9f5eef7c6133a320d8565883e88a53a6dfb976217b2` |
| formal ABBA summary | `cf591d2605b65478ee0239deb6ae489015cc8c9056cb23b6adf4d764d745ed01` |
| lifecycle soak | `3cf9085c4259595ba84966fe08a19be33da10d62bc72a2d7da513d0ef4199c73` |
| Linux platform acceptance | `e4e72ddf3cabd507d45483304bf800d3a0ee60e3bfa2668dd54859df4dbedb89` |

MangoHud application-frame 结果如下；1% low 来自尾部分布，不能从 p99 直接推导：

| 场景/客户端 | Samples | p50 ms | p95 ms | p99 ms | 1% low FPS |
| --- | ---: | ---: | ---: | ---: | ---: |
| overall Vanilla | 56,496 | 3.440 | 5.067 | 5.859 | 162.51 |
| overall BBB | 70,245 | 2.820 | 4.294 | 4.764 | 174.10 |
| static Vanilla / BBB | 17,053 / 22,252 | 3.919 / 2.971 | 4.216 / 3.284 | 4.400 / 3.399 | 207.41 / 248.53 |
| flight Vanilla / BBB | 21,888 / 26,718 | 3.041 / 2.496 | 3.527 / 2.792 | 3.858 / 2.917 | 244.14 / 220.41 |
| boundary Vanilla / BBB | 17,555 / 21,275 | 3.488 / 2.844 | 5.725 / 4.691 | 6.167 / 4.986 | 156.09 / 160.57 |

`perf stat` 的总体进程均值为 Vanilla/BBB task-clock
`1.091/1.056 ms/frame`、cycles `4,529,822/4,544,526`、instructions
`6,899,320/6,662,667`；这两种工具口径不能相减为 CPU 或 GPU 时间。MangoHud 0.8.4
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

本轮正式两轮 ABBA 中，BBB 相对 Vanilla 的 task-clock 为 `-3.14%`，instructions 为
`-3.43%`；cycles 为 `+0.32%`，按 presented frame 如实报告但不冒充时间。BBB 的
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
分别为 `83,063/36,362`。GPU execution p50 为 `1.542/2.735 ms`，p95 为
`3.273/3.104 ms`，p99 为 `4.335/4.495 ms`。GPU trace 对 Weston surface-commit p50
的影响为 `+0.023%/+0.004%`，MangoHud 为 `-0.013%/-0.055%`，均在 1% 门槛内。

## 当前差异

capture commit `6c16fda0` 的视觉/workload/draw、正式两轮 ABBA、MangoHud/perf、
GPU-duration association 与 overhead 已全部通过，并由 full-gated `0bfd97ae0` 的
`elf-load-image-v1` bridge 和 exact-HEAD Linux/lifecycle receipt 收口；本页更新通过
checkpoint full gate 后关闭矩阵。frame interval、
进程 CPU 均值与 GPU execution duration 是三个各自独立测得的量，
任何一个都不能由另一个推导或相减得出；draw count 因 API batching 不同，同样不能作为
等负载或性能倍数。

## 账本/goal 路由

等负载比较与 GPU duration 在本页通过 checkpoint full gate 后关闭账本项。非 Linux 发布行等相邻平台状态只取
[`unsupported-features.md`](../../../unsupported-features.md)。

## 精确重开条件

采集链、fence context 配对规则、ring 归类或 runner 授权方式变化时重开；association
ratio 跌破门槛或 overhead 超阈值同样重开。

## 相关页面和源码索引

- [性能与验证边界](../../architecture/performance-and-validation.md)
- [官方离线运行体系](../../architecture/offline-runtime.md)
- [方块渲染](../rendering/block-rendering.md)、[fluid rendering](../rendering/fluid-rendering.md)
