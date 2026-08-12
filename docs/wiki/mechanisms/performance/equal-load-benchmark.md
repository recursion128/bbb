# 官方 Vanilla 26.1 等负载性能比较

机制 ID：`performance-official-equal-load`

## 机制目的和边界

该机制在同机、同显示环境、同服务端权威相机序列和渲染一致性门禁下比较未修改的官方
客户端与 BBB。frame interval、进程 CPU 均值和 GPU execution duration 是三种独立测量，
不能互相推导或相减；draw count 受 API batching 影响，也不能直接作为等负载或性能倍数。

## Vanilla 26.1 权威实现

离线启动见[offline runtime](../../architecture/offline-runtime.md)。相机路径是
`TeleportCommand.performTeleport` → `ServerPlayer.teleport` →
`ServerGamePacketListenerImpl.teleport` → `ClientboundPlayerPositionPacket`，由
`ClientPacketListener.handleMovePlayer` 消费。官方 jar 不 patch，不加载 mod 或 agent。

## 决策输入与状态所有权

version/assets/world/resource/replay/settings/scene hash 定义运行身份；server tick 拥有相机
pose，客户端只消费 packet。compositor、GPU、kernel、电源和工具版本属于 host evidence。
仓库拥有 schema、fixture、generator、parser、validator 和当前 baseline receipt；run raw
artifact 放在 capture 目录，只有被 receipt/manifest 哈希绑定的当前报告进入版本库。

## 判断与处理顺序

1. 校验官方 jar、libraries/natives/assets，生成固定 options 和隔离 gameDir。
2. 从只读 source world 创建 run copy，加载固定 datapack/replay。
3. 充分 warm-up 后采集 `static_orbit`、`preloaded_flight`、
   `chunk_boundary_crossing` 三场景。
4. 同 tick 核对 pose、camera、settings、chunk/section/entity/particle、material ROI 和
   visible/resident/draw workload；任一差异使 run 作废。
5. 在 clean full-gated commit 上冻结 binary/load-image、dependency 和 runtime receipt。
6. BBB candidate 先做 B→C→C→B ×2；所有硬门禁 scope 通过后才做
   Java→BBB→BBB→Java ×2。
7. parser 先拒绝 hash、correction、resolution、content、throttle、hook、receipt 和 scope
   缺失，再聚合 raw；combined 不能掩盖 scene、round 或 nearest-pair 失败。
8. 最终报告保存每个 run、无效原因、截图/marker/residency、共同 draw/frame evidence、
   工具版本和 SHA-256。

## 性能与稳定性判据

对 BBB candidate，硬门禁是相对当前 baseline 的 mean FPS、App/GPU absolute p99、App
1% low、App/GPU hitch 与相同 instrumentation 的 input software latency。FPS margin 必须由
至少四次重复噪声预先派生，最大 1%；candidate 即使仍快于官方，也不能豁免自身回退。

对官方 Java，BBB mean FPS 不得更低，App p99 不得更高，1% low 不得更低，App/GPU hitch
不得更多。App mean frame/p50/p95/stddev/p99-p50/CV 与 GPU mean/p50/p95/stddev/
p99-p50/CV 始终逐 scope 记录为诊断；只有这些字段略差时不产生 blocker，伴随硬指标回退
时随硬指标失败报告。禁止 sleep、固定 pacing、FPS cap、VSync、降低分辨率或减少可见工作量。

## 测量面

### application frame 与进程 CPU

frame interval 取实际 present hook 的逐帧序列；1% low 从尾部分布计算，不能用 p99 倒数
替代。`perf stat` 或平台等价进程计数只按 presented frame 归一为均值，不冒充 per-frame
CPU percentile，也不与 frame/GPU duration 相减。

### GPU duration

Linux 的官方 OpenGL/radeonsi 与 BBB Vulkan/RADV 共用 amdgpu submit 链：
`amdgpu_cs_ioctl` → `amdgpu_sched_run_job` → `dma_fence_signaled`。parser 用客户端 PID
和 `(context, seqno)` 归属作业，并区分 scheduler 的 scheduled/finished 相邻 context；
association ratio 不足或工具 overhead 超 1% 时不发布该指标。

macOS BBB 使用 wgpu timestamp query。普通 profiler 保持最小 query；显式
`renderer-gpu-stages-v1` 才扩展有序 pass boundary。stage 列总和、total GPU duration、
archive、raw sidecar 与 schema marker 必须互证；诊断路径本身不能冒充性能 candidate。

### frame phase 与 flight

`frame_phases.csv` 将 encode-before-wait、previous-submission synchronization、surface
acquire、presentation tail、queue submit、present、async GPU poll 与完整 render call 分开。
由于 render-start 看到的是刚结束的前一 interval，主相关性按 phase[N]→interval[N+1]
计算。同 row correlation 只作时序诊断。

`renderer-frame-flight-v1` 记录 render/submit 到 wgpu completion callback 被运行时观察的
年龄、callback 到 wait return、in-flight depth 和 missing/drop 计数。它只能说明软件何时
观察 completion，不能称为硬件完成、scanout 或 photon 时间。未启用 profiler 时不注册
callback，不增加热路径工作。

### input software latency

`renderer-input-latency-v1` 在输入进入 `SimulationMailbox` 时分配单调 sequence/timestamp，
依次关联 simulation apply、`RendererFrame` extract、`Queue::submit` 返回和
`SurfaceTexture::present` 调用返回。合并输入必须记录 `superseded_events`；乱序、缺列、
archive/raw 不一致或阶段和不等于 end-to-end 均 fail-closed。

终点固定为 `present-call-return-software-boundary-not-display-or-photon`，不包含 compositor、
scanout、面板或光子延迟。queue-budget/acquire candidate 必须用相同 replay 和 instrumentation
做独立配对 A/B，不能只凭 FPS 接受。

## 当前 macOS Retina 基线

当前唯一 baseline receipt 是
[`current-release-baseline.json`](../../../evidence/macos-native-retina/current-release-baseline.json)，
正式报告由该文件的路径与 SHA-256 绑定。候选工具不得在源码或 Wiki 另写
commit/hash/FPS 副本；必须读取 receipt，并验证 registry wgpu/core/hal/types/naga 30.0.0、
binary hash、`PassBoundary`、Immediate/hint 2、实际三个 drawable、单 in-flight、fullscreen
2560x1600 和 workload/input identity。兼容 schema 只定义 parser 行为，不赋予非当前
report 当前状态；其演进从 Git history 追溯。

## 生命周期和 fallback

所有 client/server/compositor/tool 进入独立 process group 并 bounded cleanup。图形 hook、
GPU trace 或 profiler overhead 超 1%，或 GPU association 不足时，该工具数据无效；不得降格
猜测。surface lost/outdated/timeout/occluded/suboptimal 按运行时合同 fail-closed 或 reconfigure，
不能把错误帧混入正式样本。

## BBB 实现映射

`scripts/official_client_benchmark.py` 拥有官方 audit、launch、replay、smoke、capture parsing
和 summary；`scripts/macos_native_retina.py` 拥有 Retina detect/run/revalidate/candidate-check；
profile fixture 冻结场景与报告合同。BBB runtime 只发布只读 pose、visible/resident、draw 和
opt-in timing diagnostics，不用 control plane 驱动正式相机。

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 官方离线启动 | `Main` / `Minecraft` offline branches | benchmark launcher | `partial` | jar/native/options receipt |
| 共同相机 replay | player-position packet path | replay + world apply | `partial` | pose/camera hash |
| 可见 workload | official screenshot/draw | BBB counters + ROI/draw evidence | `partial` | 三场景 identity gate |
| frame interval | OpenGL/Metal presentation | parser/summary | `partial` | raw ABBA |
| CPU process means | client PID accounting | normalization parser | `partial` | event/permission receipt |
| GPU duration | platform GPU submission chain | GPU parser/timestamp queries | `partial` | association/raw sidecar |

状态取 parity matrix 的 current release receipt；本页不独立关闭这些行。

## 结构性不变量

- 相机、画面、资源、输入和 visible/resident/draw workload 必须先一致。
- 正式采样来自 clean full-gated binary，且每 run 绑定相同 capture receipt。
- 每客户端至少四次有效 capture；正式比较保留完整两轮 ABBA。
- 任何 profiler、hook 或 trace overhead 超阈值都会使对应指标无效。
- 性能优化不能改变可见 workload、submission 身份或输入终点定义。

## 聚焦测试（focused tests）和真实证据

`scripts.tests.test_official_client_benchmark` 覆盖启动身份、replay、percentile、1% low、hitch、
normalization、无效原因和 cleanup。`scripts.tests.test_macos_native_retina` 覆盖 panel-native/
drawable 先行拒绝、dependency/runtime receipt、raw/archive 互证、统计派生、CV-only 非阻塞、
硬指标回退、逐 scope、baseline FPS 非劣、frame-phase/flight/GPU/input schema 与错误路径。

## 当前差异

当前 release identity 的开放 validation class 只取 machine-readable acceptance report；
本页不复制其数量或时点。

## 账本/goal 路由

当前 row 是否 `equivalent`、`partial` 或 active 只取 parity matrix、acceptance report 与
[`unsupported-features.md`](../../../unsupported-features.md)。

## 精确重开条件

release binary、schema、world/resource/replay/settings/input、workload/draw、surface、
instrumentation、association 或 overhead 合同变化时，由 acceptance planner 选择必须重跑的
validation class。任何 baseline receipt 身份变化都要求重新校验所有直接依赖它的 candidate。

## 相关页面和源码索引

- [性能与验证边界](../../architecture/performance-and-validation.md)
- [官方离线运行体系](../../architecture/offline-runtime.md)
- [方块渲染](../rendering/block-rendering.md)、[fluid rendering](../rendering/fluid-rendering.md)
