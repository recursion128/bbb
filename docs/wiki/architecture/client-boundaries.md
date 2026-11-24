# 客户端边界与状态分类

## 四类状态

| 类别 | 定义 | 典型 owner | 发布规则 |
| --- | --- | --- | --- |
| canonical state | 所属语义的唯一可变客户端状态 | `bbb-client-world::WorldStore`、native `Screens`（由 `ClientInputState` 持有）、resource generation | 只在所属线程/阶段变更 |
| derived snapshot | 为 control、renderer、诊断提取的只读投影 | `RendererFrame`、`ControlSnapshot` | 在一致边界提取或替换，不回写 live state |
| transient observed event | 声音、粒子、一次性 UI 等有序副作用 | `PlayApplyEffects` 实现 | exactly-once 顺序消费，不伪装成字段 |
| backend runtime state | socket、GPU buffer、audio voice、window handle | net/renderer/audio/native runtime | 后端生命周期内有效，重建不改变游戏真值 |

## 当前 crate 边界

客户端有八个专属包与宿主机 Web 网关：`bbb-client` 编排主循环、输入、UI、`terrain_upload`、`item_models::tooltip`、control 与本地/浏览器存储（同时提供原生桌面二进制与 `wasm32` Web 客户端 `cdylib`）；
`bbb-client-net` 与 world/assets 解耦，拥有连接与录制（原生 TCP 与浏览器二进制 WebSocket 共用同一状态机）、Services 公钥以及异步 Mojang profile/skin HTTP 服务（`profile_textures`）；
`bbb-client-assets` 拥有资源字节、`virtual_store` 与加载时解析；
`bbb-client-world` 拥有可见世界、菜单事实、ICU 本地时间（`local_time`）、方块轮廓几何（`block_outline`）与预测（扫掠碰撞复用 `bbb_facts::collision`）；
`bbb-client-render` 拥有 GPU 资源、跨桌面与 Web 统一的纯 Rust `ab_glyph` 字体光栅与提交，不依赖 `bbb-client-world`、网络 HTTP、FFI 或 `icu_*`（零 `WorldStore`/网络反向边）；
`bbb-client-audio` 拥有原生（`NativeAudioRuntime` + OpenAL 生产后端；`kira` 仅限测试）与浏览器（`webaudio` 离主线程解码）播放后端；
`bbb-client-clock` 提供零 workspace 依赖的客户端及回放时钟；
`bbb-client-probe` 拥有无头差分/会话 `probe` harness 与 `bbb-replay-diff` CLI；
`bbb-web-host` 提供浏览器跨源隔离静态宿主、WebSocket↔TCP 桥接与反向 MCP/control 隧道（仅依赖 `bbb-protocol`、`bbb-client-assets` 与 `bbb-mcp`，零 `bbb-client`/`wgpu`/`winit` 依赖；详见[Web 客户端与网关](../development/web-client.md)）。
wire、固定事实与碰撞几何、registry codec、共享渲染值与 WASM 同步原语（`bbb-render-types::wasm_sync`）及 MCP 传输分别使用共享的 `bbb-protocol`、
`bbb-facts`、`bbb-registry-codec`、`bbb-render-types` 和 `bbb-mcp`。
客户端本地光照复用 `bbb-protocol::chunk_light` 的纯传播核心与临时存储，实体体转向复用
`bbb-protocol::entity_body_rotation`，实体默认属性复用 `bbb-facts::default_attributes_26_1`；
光照真值仍由 `WorldStore` 的 `ChunkColumn.light` 唯一拥有，客户端 crate 不再依赖任何 `bbb-server-*` crate。

世界事实由 `WorldStore` 拥有，活动屏幕及其编辑状态由 native `Screens` 拥有。
后者保存当前 `Screen` 与实际返回屏；文本、焦点、拖拽、滚动和 IME 不经 world 镜像或
publication 回写。菜单数据、资源 generation 和屏幕存在性分别由实际 owner 决定。

## 所有权规则

- 一项长期事实只有一个 semantic owner。消费者持有 id、handle 或 projection，不维护
  可独立漂移的副本。
- 内置物品原型由 `bbb-facts::vanilla_item_prototype_component_catalog` 提供；
  `bbb-client-assets::ItemRegistryCatalog` 从共享原型及物品、方块事实派生查询与展示目录，
  组件 override 统一使用完整 codec 值，组件是否存在直接取其键，不再保存、传递平行类型集合或标量表。
  world 的强类型原型组件统一拥有运行时默认值：
  静态组件在注册表发布前可用；holder 和 tag 绑定随候选
  frozen registry 发布，重连时清除旧绑定。renderer 与本地交互读取同一份组件；装备槽位与紧凑
  asset 索引在发布时派生，实体许可按当前标签解析。资源 generation 与 asset 名称 intern 独立保留。
  长矛的第一、第三人称动画共用有效 stack 动能组件查询，显式覆盖与移除优先于原型。
  属性修饰器按内建属性 ID 解析一次；tooltip 和模型谓词借用有效 stack 的同一份列表，嵌套物品沿用
  相同 profiles。隐藏和自定义显示数据保留到各自的消费边界。
  默认与显式工具共用有序规则和当前方块标签；挖掘查询只解析一次目标 holder ID，速度与掉落各取首个匹配值。
  默认工具的 holder 集合在发布与方块标签更新时绑定；查询直接借用现有 ID 集合。工具速度应用玩家修饰器后才量化为挖掘进度。
- 内建方块破坏、遮挡与声音事实由 `bbb-facts` 拥有。world 按合法 state id 查询破坏和音效，
  放置声按 block type id 查询；assets 不维护内建方块规则目录，资源重载不复制这些固定规则。
- protocol 类型描述 wire；world 类型描述客户端语义。不能把原始 packet 当持久世界。
- renderer input 必须携带 Vanilla 决策需要的独立事实。不能从
  `Opaque/Cutout/Translucent`、方块名称或 render shape 猜测 `canOcclude`、
  `isSolidRender`、`isViewBlocking`、光衰减、shade、shape、`skipRendering` 或 fluid
  overlay。
- reload 在 renderer 内完成资源 generation；simulation 确认当前 generation 后先解码并应用 world
  默认值，再确认加载成功。解码失败保留旧 world 默认值，并进入已有的上一资源 stack 恢复流程。
- 本地预测必须可被服务端 packet 校正；诊断 snapshot 不得成为控制输入。
- world→renderer 只经一条 frame-extract/submission 路径；`Gui` 的直接 support accessor
  不成为第二个决策 owner。客户端每帧流式 GPU 数据复用持久 buffer，不每帧 `create_buffer_init`。
- clientbound play packet 只经 `bbb-client-world::apply_play_packet` 进入 world，probe 与正式客户端共用同一路径；
  实体与粒子 type id 只取 `bbb_protocol` catalog，server 下发的 dynamic registry 是会话唯一权威（Holder/HolderSet 统一接 resolver）；
  玩法状态与渲染动画只读注入的时钟和 RNG，control/MCP 的写请求复用玩家输入与交互同一路径。
- terrain/item atlas、block-entity client effect 与 audio runtime 各归所属语义域；
  `LevelEventHandler` 的 switch 不成为第二个机制 owner。
- 模组实体/方块/物品目录随 Configuration `bbb:content_registry` 进入 `WorldStore` 只读会期快照（`ContentRegistry`）；`bbb-client-assets::EntityRigCatalog` 从资源包或内建表解析 data-driven 骨骼与纹理；`bbb-client-render` 统一在保留默认 Vanilla 无损耗的前提下按帧参数消费动态光源与 horizon LOD；纯客户端 QoL（`--ext`）由 `bbb-client` 消费启动旗标并沿共享 tooltip / FOV / HUD debug 通道投影，无游戏内配置界面。

## 核心机制入口

| 机制 | canonical 页面 | 跨边界合同 |
| --- | --- | --- |
| 输入、移动与交互 | [客户端输入与交互](../mechanisms/player/client-input-and-interaction.md) | raw input、world latch、outbound baseline 分属各 owner；prediction 先取得发送 permit |
| level、chunk/light、entity 与 session 服务 | [客户端世界生命周期](../mechanisms/world/client-world-lifecycle.md) | level generation 与 player load identity 分离；consumer 只读投影 |
| simulation、control、reload 与退出 | [客户端运行时](../mechanisms/runtime/client-runtime-and-control.md) | 主线程 step、唯一命令 FIFO、同代只读 control 发布与统一 join |
| seed、音源与播放 | [客户端音频链路](../mechanisms/audio/client-audio-pipeline.md) | world → resolver → backend；FIFO 与 generation 不丢失 |
| 连接与注册表 | [客户端连接](../mechanisms/protocol/client-connection-and-registries.md) | frame/phase 与 world registry validation 分离，完成验证后才确认配置 |
| 场景与 GPU 呈现 | [客户端场景与渲染](../mechanisms/rendering/client-scene-and-rendering.md) | typed frame、ordered layers 与 resource generation，renderer 不反查业务状态 |
| screen、HUD 与背包 | [客户端屏幕](../mechanisms/ui/client-screens-hud-and-inventory.md) | world 菜单事实、native Screens/共享布局、renderer GPU 表面分开，控制复用真实 action |
| 聊天、社交与 profile | [客户端聊天与身份](../mechanisms/security/client-chat-and-identity.md) | 验证、显示、服务 availability 与 texture CPU/GPU 生命周期分别拥有 |
| 本地文件与平台 | [客户端配置与窗口](../mechanisms/persistence/client-options-and-platform.md) | CLI 显式覆盖、文件语义与设备观察分开，配置仅启动提供 |

处理顺序、失败边界与测试只在对应机制页维护；本页拥有跨 crate 和状态分类合同。

## 测试责任

protocol 测 byte/类型边界，world 测 apply 后可查询的事实，native 测 screen 动作、
publication 与消费者接线，renderer/audio 测后端输入合同。跨边界测试证明可观察的数据和输出；
协议与实际事件所需的顺序在对应边界验证，不重复每层内部测试。音频单元测试不依赖真实音频设备。

<a id="mojang-client-telemetry"></a>

## Mojang 客户端遥测边界

BBB 不收集或上传 Mojang client telemetry。Vanilla 26.1 的
`WorldSessionTelemetryManager` 会把 server brand 写入 `WorldLoadEvent` 并尝试通过独立
event sender 发送；这不是 multiplayer protocol canonical state。BBB 仍可把对应 wire
字段解码为诊断事实，但不得为它创建外部 telemetry runtime owner。只有项目明确把该
外部服务纳入产品表面时才重新审计；当前裁决由
[缺口账本](../../unsupported-client-features.md)唯一维护。
