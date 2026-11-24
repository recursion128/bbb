# 客户端边界与状态分类

## 四类状态

| 类别 | 定义 | 典型所有者 | 发布规则 |
| --- | --- | --- | --- |
| 权威状态 | 所属语义的唯一可变客户端状态 | `bbb-client-world::WorldStore`、原生 `Screens`（由 `ClientInputState` 持有）、资源代次 | 只在所属线程/阶段变更 |
| 派生快照 | 为控制、渲染器与诊断提取的只读投影 | `RendererFrame`、`ControlSnapshot` | 在一致边界提取或替换，不回写活跃状态 |
| 瞬时可观察事件 | 声音、粒子、一次性 UI 等有序副作用 | `PlayApplyEffects` 实现 | 精确一次顺序消费，不伪装成持久字段 |
| 后端运行时状态 | 套接字、GPU 缓冲区、音频声道、窗口句柄 | 网络/渲染/音频/原生运行时 | 在后端生命周期内有效，重建不改变游戏真值 |

## 当前 crate 边界

客户端由八个专属 crate 与宿主机 Web 网关构成：`bbb-client` 编排主循环、输入、UI、`terrain_upload`、`item_models::tooltip`、控制与本地/浏览器存储（同时提供原生桌面二进制与 `wasm32` Web 客户端 `cdylib`）；`bbb-client-net` 与世界/资产层解耦，拥有连接与录制（原生 TCP 与浏览器二进制 WebSocket 共用同一状态机）、Services 公钥以及异步 Mojang 档案/皮肤 HTTP 服务（`profile_textures`）；`bbb-client-assets` 拥有资源字节、`virtual_store` 与加载时解析；`bbb-client-world` 拥有可见世界、菜单事实、ICU 本地时间（`local_time`）、方块轮廓几何（`block_outline`）与预测（扫掠碰撞复用 `bbb_facts::collision`）；`bbb-client-render` 拥有 GPU 资源、跨桌面与 Web 统一的纯 Rust `ab_glyph` 字体光栅化与提交，不依赖 `bbb-client-world`、网络 HTTP、FFI 或 `icu_*`（零 `WorldStore`/网络反向边）；`bbb-client-audio` 拥有原生（`NativeAudioRuntime` + OpenAL 生产后端；`kira` 仅限测试）与浏览器（`webaudio` 离主线程解码）播放后端；`bbb-client-clock` 提供零工作区依赖的客户端及回放时钟；`bbb-client-probe` 拥有无头差分/会话 `probe` 工具与 `bbb-replay-diff` CLI；`bbb-web-host` 提供浏览器跨源隔离静态宿主、WebSocket↔TCP 桥接与反向 MCP/控制隧道（仅依赖 `bbb-protocol`、`bbb-client-assets` 与 `bbb-mcp`，零 `bbb-client`/`wgpu`/`winit` 依赖；详见[Web 客户端与网关](../development/web-client.md)）。
线协议、固定事实与碰撞几何、注册表编解码器、共享渲染值与 WASM 同步原语（`bbb-render-types::wasm_sync`）及 MCP 传输分别使用共享的 `bbb-protocol`、`bbb-facts`、`bbb-registry-codec`、`bbb-render-types` 和 `bbb-mcp`。
客户端本地光照复用 `bbb-protocol::chunk_light` 的纯传播核心与临时存储，实体体转向复用 `bbb-protocol::entity_body_rotation`，实体默认属性复用 `bbb-facts::default_attributes_26_1`；光照真值仍由 `WorldStore` 的 `ChunkColumn.light` 唯一拥有，客户端 crate 不依赖任何 `bbb-server-*` crate。

世界事实由 `WorldStore` 拥有，活动屏幕及其编辑状态由原生 `Screens` 拥有。后者保存当前 `Screen` 与实际返回屏；文本、焦点、拖拽、滚动和输入法不经世界镜像或快照发布回写。菜单数据、资源代次和屏幕存在性分别由各自实际所有者决定。

## 所有权规则

- 一项长期事实只有一个语义所有者。消费者仅持有 ID、句柄或只读投影，不维护可独立漂移的副本。
- 内置物品原型由 `bbb-facts::vanilla_item_prototype_component_catalog` 提供；`bbb-client-assets::ItemRegistryCatalog` 从共享原型及物品、方块事实派生查询与展示目录，组件覆盖统一使用完整编解码值，组件是否存在直接取其键，不再保存、传递平行类型集合或标量表。世界层的强类型原型组件统一拥有运行时默认值：静态组件在注册表发布前可用；holder 和标签绑定随候选冻结注册表发布，重连时清除旧绑定。渲染器与本地交互读取同一份组件；装备槽位与紧凑资产索引在发布时派生，实体许可按当前标签解析。资源代次与资产名称驻留表独立保留。长矛的第一、第三人称动画共用有效物品堆叠动能组件查询，显式覆盖与移除优先于原型。属性修饰器按内建属性 ID 解析一次；提示框和模型谓词借用有效物品堆叠的同一份列表，嵌套物品沿用相同档案规则。隐藏和自定义显示数据保留到各自的消费边界。默认与显式工具共用有序规则和当前方块标签；挖掘查询只解析一次目标 holder ID，速度与掉落各取首个匹配值。默认工具的 holder 集合在发布与方块标签更新时绑定；查询直接借用现有 ID 集合。工具速度应用玩家修饰器后才量化为挖掘进度。
- 内建方块破坏、遮挡与声音事实由 `bbb-facts` 拥有。世界层按合法状态 ID 查询破坏和音效，放置声按方块类型 ID 查询；资产层不维护内建方块规则目录，资源重载不复制这些固定规则。
- 协议类型描述线协议；世界类型描述客户端语义，不能把原始数据包当作持久世界。
- 渲染器输入必须携带 Vanilla 决策需要的独立事实，不能从 `Opaque`/`Cutout`/`Translucent`、方块名称或渲染形状猜测 `canOcclude`、`isSolidRender`、`isViewBlocking`、光衰减、明暗、形状、`skipRendering` 或流体覆盖面。
- 重载在渲染器内完成资源代次切换；模拟层确认当前代次后先解码并应用世界层默认值，再确认加载成功。解码失败保留旧世界默认值，并进入已有的上一资源栈恢复流程。
- 本地预测必须可被服务端数据包校正；诊断快照不得成为控制输入。
- 世界到渲染器只经一条帧提取与提交路径；`Gui` 的直接辅助查询不成为第二个决策所有者。客户端每帧流式 GPU 数据复用持久缓冲区，不每帧调用 `create_buffer_init`。
- 服务端下发的游玩数据包只经 `bbb-client-world::WorldStore::apply_play_packet` 进入世界，无头探针与正式客户端共用同一路径；实体与粒子类型 ID 只取 `bbb_protocol` 目录，服务端下发的动态注册表是会话唯一权威（`Holder`/`HolderSet` 统一接解析器）；玩法状态与渲染动画只读注入的时钟和随机数生成器，控制/MCP 的写请求复用玩家输入与交互同一路径。
- 地形/物品图集、方块实体客户端效果与音频运行时各归所属语义域；`LevelEventHandler` 的分派不成为第二个机制所有者。
- 模组实体/方块/物品目录随配置阶段 `bbb:content_registry` 进入 `WorldStore` 只读会期快照（`ContentRegistry`）；`bbb-client-assets::EntityRigCatalog` 从资源包或内建表解析数据驱动骨骼与纹理；`bbb-client-render` 统一在保留默认 Vanilla 无损耗的前提下按帧参数消费动态光源与远景 LOD；纯客户端体验扩展（`--ext`）由 `bbb-client` 消费启动参数并沿共享提示框 / FOV / HUD 调试通道投影，无游戏内配置界面。

## 核心机制入口

| 机制 | 规范页面 | 跨边界合同 |
| --- | --- | --- |
| 输入、移动与交互 | [客户端输入与交互](../mechanisms/player/client-input-and-interaction.md) | 原始输入、世界状态锁存、出站基线分属各所有者；预测先取得发送许可 |
| 关卡、区块光照、实体与会话服务 | [客户端世界生命周期](../mechanisms/world/client-world-lifecycle.md) | 关卡代次与玩家加载身份分离；消费者只读投影 |
| 模拟、控制、重载与退出 | [客户端运行时](../mechanisms/runtime/client-runtime-and-control.md) | 主线程步进、唯一命令 FIFO、同代只读控制发布与统一线程汇合 |
| 种子、音源与播放 | [客户端音频链路](../mechanisms/audio/client-audio-pipeline.md) | 世界 → 音源解析 → 音频后端；FIFO 顺序与资源代次不丢失 |
| 连接与注册表 | [客户端连接](../mechanisms/protocol/client-connection-and-registries.md) | 帧/阶段状态机与世界注册表校验分离，完成校验后才确认配置 |
| 场景与 GPU 呈现 | [客户端场景与渲染](../mechanisms/rendering/client-scene-and-rendering.md) | 强类型帧输入、有序图层与资源代次，渲染器不反查业务状态 |
| 屏幕、HUD 与背包 | [客户端屏幕](../mechanisms/ui/client-screens-hud-and-inventory.md) | 世界菜单事实、原生 `Screens`/共享布局、渲染器 GPU 表面分开，控制复用真实交互动作 |
| 聊天、社交与档案 | [客户端聊天与身份](../mechanisms/security/client-chat-and-identity.md) | 签名验证、显示、服务可用性与纹理 CPU/GPU 生命周期分别拥有 |
| 本地文件与平台 | [客户端配置与窗口](../mechanisms/persistence/client-options-and-platform.md) | CLI 显式覆盖、文件语义与设备观察分开，配置仅在启动时提供 |

处理顺序、失败边界与测试只在对应机制页维护；本页拥有跨 crate 和状态分类合同。

## 测试责任

协议层测字节与类型边界，世界层测数据包应用后可查询的事实，原生运行时测屏幕动作、快照发布与消费者接线，渲染/音频层测后端输入合同。跨边界测试证明可观察的数据和输出；协议与实际事件所需的顺序在对应边界验证，不重复每层内部测试。音频单元测试不依赖真实音频设备。

<a id="mojang-client-telemetry"></a>

## Mojang 客户端遥测边界

BBB 不收集或上传 Mojang 客户端遥测数据。Vanilla 26.1 的 `WorldSessionTelemetryManager` 会把服务端品牌名写入 `WorldLoadEvent` 并尝试通过独立事件发送器上报；这不属于多人协议权威状态。BBB 仍可把对应线协议字段解码为诊断事实，但不得为它创建外部遥测运行时所有者。只有项目明确把该外部服务纳入产品范围时才重新审计；当前裁决由[客户端功能差异](../../unsupported-client-features.md)唯一维护。
