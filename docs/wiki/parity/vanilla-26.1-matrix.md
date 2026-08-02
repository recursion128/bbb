# Vanilla 26.1 机制等价矩阵

本表是系统审计入口，不是 feature ledger。`Route` 中的未关闭项链接唯一状态 owner；
`—` 表示已有稳定证据且无需工程排程。

| Mechanism ID | 领域 | Vanilla owner/class | BBB owner/module | Required state/projection | 状态 | Focused test/evidence | Route / reopen condition |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `startup-offline-identity` | startup | `Main`, `Minecraft`, `User` | benchmark launcher/native CLI | username, offline UUID/services | `equivalent` | [offline runtime](../architecture/offline-runtime.md) | 升级版本时重审 |
| `resources-offline-asset-trust` | resources | asset index/resource manager | `bbb-pack`, benchmark audit | metadata, SHA-1 object map | `equivalent` | [机制页](../mechanisms/resources/offline-asset-trust.md) | index/metadata codec 变化时重开 |
| `resources-pack-stack-reload` | resources | pack repository/reload instance | `bbb-pack::ClientResourceGeneration` | ordered providers, generation | `equivalent` | pack lifecycle tests | 新 consumer 绕过 generation 时重开 |
| `protocol-phase-lifecycle` | protocol | `GameProtocols`, packet listeners | `bbb-net::ConnectionState` | phase, typed packet order | `equivalent` | packet coverage 256 closed | 目标 protocol 变化时重开 |
| `protocol-client-state-flow` | protocol/world | `ClientPacketListener`, `ClientLevel` | `WorldStore::apply_play_packet` | registry context, ordered events | `equivalent` | [机制页](../mechanisms/protocol/client-state-flow.md) | packet table 变化时重开 |
| `registries-synchronized-data` | registries | `RegistryDataLoader` | world registry loaders | dynamic holders/session ids | `equivalent` | registry coverage 194 | synchronized set 变化时重开 |
| `registries-fixed-client-catalogs` | registries | `BuiltInRegistries` | generated protocol catalogs | fixed id/name tables | `equivalent` | [registry inventory](../../registry-data-inventory-26.1.md) | bootstrap order 变化时重开 |
| `world-chunk-canonical-state` | world | `ClientLevel`, chunk cache | `bbb-world::chunks` | block/light/biome/block entity | `equivalent` | chunk/play-apply tests | packet/section layout 变化时重开 |
| `world-entity-canonical-state` | entities | `ClientLevel`, `Entity` | `bbb-world::entities` | spawn, metadata, equipment | `equivalent` | entity coverage 157 | entity network surface 变化时重开 |
| `world-local-player-prediction` | movement | `LocalPlayer` | native/world movement owners | input, collision, ack sequence | `equivalent` | movement/collision focused tests | authoritative type变化时重开 |
| `rendering-block-model-pipeline` | rendering | `ModelBlockRenderer`, `BlockModelLighter` | terrain projection/mesh | exact occlusion/light facts | `equivalent` | [方块机制页](../mechanisms/rendering/block-rendering.md) | 关键 ROI 或 state table 变化时重开 |
| `rendering-chunk-layer-routing` | rendering | `SectionCompiler` | terrain renderer | material layer, sort state | `equivalent` | layer/anchor evidence | render layer bootstrap 变化时重开 |
| `rendering-fluid-mesh` | rendering | `FluidRenderer` | terrain fluid mesh | fluid/neighbor/overlay/tint/light | `equivalent` | [fluid 机制页](../mechanisms/rendering/fluid-rendering.md) | fluid ROI 或 renderer 变化时重开 |
| `rendering-camera-fluid-fog` | rendering | `Camera.getFluidInCamera`, fog environments | native camera/fog state | None/Water/Lava + effects | `equivalent` | camera environment tests/archive | camera/fog owner 变化时重开 |
| `rendering-entity-models` | entities/rendering | entity renderers/states/layers | world/native/renderer entity pipeline | model, pose, material, light | `equivalent` | entity coverage + anchors | 新 entity type/version 时重开 |
| `rendering-block-entities` | rendering | block entity renderers | native/renderer block entities | typed block-entity state | `equivalent` | block-entity focused tests | new current BE surface 时重开 |
| `rendering-particles-events` | rendering | particle engine/level event handler | world effects/native/renderer | ordered transient particles | `equivalent` | particle matrix in ledger appendix | provider set 变化时重开 |
| `ui-font-provider-pipeline` | ui/text | `FontManager`, `FontSet`, providers | item-model/render-types/native | provider graph, glyph atlas | `equivalent` | [字体机制页](../mechanisms/ui/font-and-text.md) | font codec/consumer 变化时重开 |
| `ui-bidirectional-shaping` | ui/text | language/font text pipeline | native text layout | bidi runs, shaped glyph order | `equivalent` | archived shaping design/tests | shaping owner 变化时重开 |
| `ui-local-time-icu` | ui/text | ICU `SimpleDateFormat` path | local-time item property | locale/calendar/timezone data | `deferred-external` | [ledger](../../unsupported-features.md) | approved Rust-1.80-compatible ICU provider 可覆盖任意 pattern 时 |
| `ui-context-entity-type` | items/ui | `ContextEntityType.get`, resolver living owner | item model consumers | optional living owner type | `deferred-external` | [ledger](../../unsupported-features.md) | 当前新 consumer 传 real living owner 时 |
| `ui-screen-hud-state` | ui | screens/HUD/debug overlay | native/render-types/renderer | canonical screen + HUD snapshot | `equivalent` | UI coverage/archive tests | 新声明 screen surface 时重开 |
| `input-native-routing` | input | keyboard/mouse handlers | native input/control/net | focus, bindings, local intent | `equivalent` | input/playable tests | platform/input surface 变化时重开 |
| `audio-event-playback` | audio | `SoundManager`, client level sounds | world effects/audio runtime | sound holder, position, category | `equivalent` | audio runtime tests/live gate | device/backend 变化时重开 |
| `persistence-server-options` | persistence | options/server list/local storage | native persistence modules | typed files, atomic recovery | `equivalent` | persistence focused tests | schema 变化时重开 |
| `performance-official-equal-load` | performance | official client + server teleport | benchmark harness | replay/world/resource/settings hashes | `equivalent` | [等负载机制页](../mechanisms/performance/equal-load-benchmark.md) | 渲染合同/host/tool 变化时重开 |
| `performance-gpu-execution-duration` | performance | OpenGL/radeonsi and Vulkan/RADV queues | external capture boundary | client context submit→fence chain | `deferred-external` | [ledger](../../unsupported-features.md) | 两 API 获得可排除 compositor 的共同关联时 |
| `platform-linux-wayland` | platform | GLFW/winit platform surface | native platform + live gate | window/input/clipboard/IME | `equivalent` | live playable stamp | backend 变化时重开 |
| `platform-linux-x11-seat-matrix` | platform | Linux X11/compositor inputs | no maintained runner | interactive event matrix | `deferred-external` | [ledger](../../unsupported-features.md) | maintained interactive X11/seat runner 可执行同 release matrix 时 |
| `platform-macos-windows-release` | platform | native platform backends | no maintained runner | winit/wgpu/audio/input fixtures | `deferred-external` | [ledger](../../unsupported-features.md) | maintained macOS/Windows runner 可执行 release matrix 时 |
| `debug-integrated-server-mirrors` | debug | integrated server debug renderers | external client has no server owner | local server/chunk/entity mirror | `not-needed` | [ledger](../../unsupported-features.md) | repo 获得 in-process integrated server 时 |
| `resources-world-list-favicons` | resources/ui | server/world list screens | screens out of scope | external dynamic textures | `not-needed` | [ledger](../../unsupported-features.md) | 对应 screen 进入声明范围时 |
| `movement-remote-type-exceptions` | movement | rabbit/fox/bee/stray etc. local entity paths | server-owned remote entities | exact local movement authority | `not-needed` | [ledger](../../unsupported-features.md) | 列出的 exact type 获得本地权威时 |
| `telemetry-mojang-upload` | diagnostics | telemetry managers | no external sender | account/session telemetry | `not-needed` | [ledger](../../unsupported-features.md) | repo 明确加入该外部 surface 时 |
| `docs-chinese-mechanism-wiki` | workflow | local Vanilla source corpus | `docs/wiki`, docs lint | inventory, ids, routes, links | `active` | [工程合同](../../wiki-engineering-design.md) | [ledger](../../unsupported-features.md)；本 slice 关闭后改为 `equivalent` |

## 审计摘要

矩阵覆盖 startup、resources、protocol、registries、world、movement、rendering、entities、
UI/text、input、audio、persistence、performance、platform、diagnostics 和 workflow。更细的
closed packet/entity/item/provider 证据由生成 coverage 与 archive 索引承载，不在此扩成
第二份完成清单。
