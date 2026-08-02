# 官方 Vanilla 26.1 等负载性能比较

机制 ID：`performance-official-equal-load`

## 机制目的和边界

该机制在同机、同 compositor、同服务端权威相机序列和渲染一致性门禁下，比较未修改
官方客户端与 BBB。它提供 end-to-end frame interval 和进程 CPU 均值；不把 present
interval 或 GPU telemetry 冒充 GPU execution duration。

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
MangoHud 对任一 API hook 失败或自身 median overhead >1% 时不作正式结果。无法建立
API-neutral GPU context 链时只报告共同 frame interval 与 GPU load，不降格猜测。

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
| per-client GPU duration | driver/API context tracing | 无共同可验证 owner | `deferred-external` | [ledger](../../../unsupported-features.md) |

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

当前 host 的已发布摘要记录 4 次有效 capture/客户端和两轮 ABBA，八次 run 均为零
position correction、零 `>=33.333 ms` segment hitch。正式摘要位于外部路径
`/tmp/bbb-formal-comparison-summary-v4.json`，SHA-256 为
`ced023ba8a4369b617ac801093d29fc26e56d04dbe3ca869ba7ff6ec0f2faa00`；原始目录是
`/tmp/bbb-formal-abba-final-v4/`。这些路径只标识本机证据，清理或换机后必须重跑，
不能从 Wiki 反向重建 raw capture。

| Contract/evidence | SHA-256 |
| --- | --- |
| replay | `5073908b5e5cd7eeca023540b9c2844e9c4d196610a2fbb67866058c5421c388` |
| source world | `59bc552530d1c13ce3d618be5669e5d99d052e01d53e99a879a44008c7ba4bca` |
| resources | `8aa8d02d6806ef9396564b83b7b64f73014ea654d8059a1b8ae6b09f3f67df0c` |
| settings | `4ca64c1649a9ada45f8d9d7903abd488902accab7d227c5d9017b714a8ac8093` |
| scene | `6f77811b5deafacea3c7666414cc1705c396a987b015e2a5f1f4832e872f73bc` |
| server camera log | `f3749c011859c34d97f5ce0f36fdfec45e2ab7c81b51b405ad046489943a0b3a` |
| visual/workload gate | `576b14ec01439bc64c3ddcde335b387841a592cb4a006f0404423f3d07e22604` |
| RenderDoc anchor summary | `6b38c475615448aa408b28ca943f407d8bee02f3341fa042888c24f354e65a19` |

MangoHud application-frame 结果如下；1% low 来自尾部分布，不能从 p99 直接推导：

| 场景/客户端 | Samples | p50 ms | p95 ms | p99 ms | 1% low FPS |
| --- | ---: | ---: | ---: | ---: | ---: |
| overall Vanilla | 57,798 | 3.345 | 4.981 | 5.751 | 166.07 |
| overall BBB | 71,890 | 2.722 | 4.229 | 4.683 | 138.21 |
| static Vanilla / BBB | 17,120 / 22,808 | 3.903 / 2.888 | 4.171 / 3.208 | 4.361 / 3.503 | 207.70 / 194.99 |
| flight Vanilla / BBB | 22,626 / 27,476 | 2.942 / 2.393 | 3.422 / 2.676 | 3.734 / 2.881 | 251.43 / 153.75 |
| boundary Vanilla / BBB | 18,052 / 21,606 | 3.385 / 2.744 | 5.611 / 4.573 | 6.019 / 4.961 | 160.47 / 110.51 |

`perf stat` 的总体进程均值为 Vanilla/BBB task-clock
`1.083/1.309 ms/frame`、cycles `4,479,328/5,853,361`、instructions
`6,833,680/10,208,141`；这两种工具口径不能相减为 CPU 或 GPU 时间。MangoHud 0.8.4
archive SHA-256 是
`aa0b2a009552f5e7c50baabf361267a79ea5e5c0fea440786f3b202d51f57753`；RenderDoc
1.45 commit 是 `2fc0bc04cb95499635f63986a55bc6f67849dd9f`，archive SHA-256 是
`b0a7ee8ec78c4fa511eb44137380d99a748472e5fd24c877f8afcc860a172a42`。RenderDoc
只证明三个锚点的 OpenGL/Vulkan draw 内容非空；API batching 不同使 draw count 不能
作为等负载或性能倍数。

## 当前差异

GPU-only duration 尚不能跨 radeonsi OpenGL 与 RADV Vulkan 按 client context 关联
submission、queue wait、fence signal 并排除 Weston，故不得报告“GPU frame duration”。

## 账本/goal 路由

GPU duration 的 `deferred` 状态只由
[`unsupported-features.md`](../../../unsupported-features.md)维护。

## 精确重开条件

当同一无特权工具能在两 API 上提供 client-context submit→queue→fence 关联、可排除
compositor，且 overhead 通过阈值时重开。

## 相关页面和源码索引

- [性能与验证边界](../../architecture/performance-and-validation.md)
- [官方离线运行体系](../../architecture/offline-runtime.md)
- [方块渲染](../rendering/block-rendering.md)、[fluid rendering](../rendering/fluid-rendering.md)
