# 客户端场景与渲染：帧提取、实体层和资源驻留

机制 ID：`rendering-frame-order`

## 机制目的和边界

本页说明客户端如何从同一 world/input 借用视图和 resource generation 提取可呈现帧。
范围包括 camera、entity、item、particle、环境光雾、后处理和截图读回。
renderer 消费已经决定的语义状态；它不通过绘制结果反写世界，也不运行服务端实体 AI。

这里以帧顺序为主机制组织相邻领域，不把每一种实体或粒子都声明为新的覆盖单元。
[方块渲染](client-block-rendering.md)、[流体渲染](client-fluid-rendering.md)
和 [字体文本](../ui/client-font-and-text.md) 分别拥有自己的几何与采样细则。

## Vanilla 26.1 权威实现

源码根由 `scripts/vanilla path 26.1` 解析，以下位置均相对于该根。

| 决策 | 精确源码锚点 | 直接观察 |
| --- | --- | --- |
| 帧提取和呈现 | `net/minecraft/client/renderer/GameRenderer.java:400-471` | 先 window/options，再 world lightmap/camera/level，最后 GUI；render 清理深度后进入 GUI |
| 装备层顺序 | `net/minecraft/client/renderer/entity/layers/EquipmentLayerRenderer.java:54-121` | 查询选定 layer type，按声明顺序提交；首个可绘制层消费 foil，随后处理 trim |
| 顶层物品解析 | `net/minecraft/client/renderer/item/ItemModelResolver.java:38-65` | clear 输出，以 display context、level、owner 和 seed 追加模型层 |
| 粒子 tick 与提取 | `net/minecraft/client/particle/ParticleEngine.java:79-148` | 先 tick 既有组和 emitter，再接纳待加入粒子；按渲染组顺序提取；换 level 清空 |

`GameRenderer.extract` 的 `shouldRenderLevel` 同时依赖资源加载、advanceGameTime 与 level。
无 level 时仍可提取 GUI，不能把菜单帧硬塞入 world-pass 分支。
`render` 在 world 后执行 outline 和激活的 camera post effect，再进入 GUI。
源码里的自动世界截图调用点不能替代用户截图的独立输入和落盘合同。

## 决策输入与状态所有权

| 输入或状态 | BBB owner | 查询坐标与寿命 |
| --- | --- | --- |
| 实体位置、装备、动画事实 | `bbb-client-world` | entity id、level generation、simulation tick |
| camera pose、FOV、hurt/bob/spin | `bbb-client` | 当前观察实体与本帧 partial tick |
| item model 决策 | `bbb-client-render` | stack components、display context、world/owner/seed、resource generation |
| scene projection | `bbb-client` runtime | `SimulationView` 帧内借用与同代资源 |
| GPU 资源与 draw topology | `bbb-client-render` | resource generation、frame target、attachment 大小 |
| 物品静态几何驻留 | `bbb-client-render` | asset identity 与 draw target |

simulation tick 和 presentation frame 是不同的时钟。
同一世界状态可以呈现多帧；glint 的实时推进不能由“projection 未改变”优化掉。
camera 位置变化还会影响透明地形排序和依赖 camera 的实体几何。

环境查询在 native 提取侧完成。
`RendererCameraFrame` 同时携带 lightmap、fog、sky、pose、hurt、bob、spin、FOV 和 shader time。
`CameraPose` 从 yaw/pitch 旋转构造共享视图矩阵；world、frustum、第一人称手臂和 billboard
消费同一方向约定，垂直向上/向下时保留 yaw，不通过固定 up 的 look-at 重建相机基底。
第一人称睡眠相机按床朝向对齐水平视角并上移 `0.3`，同时覆盖本地玩家与被旁观的远端睡眠活体实体。
实体模型（包含本地玩家手臂）在插值位置加当前姿态眼高后采样天空光与方块光，
避免洞口或竖井中脚部与眼部光照不同时提前变暗；marker 盔甲架从非 marker 尺寸覆盖格选取最亮光照并传给附着装备。
`FirstPersonHandPresentation` 在模拟 tick 持有可见手持栈、抬手高度和转头惯性端点，
发布后由手臂与持物共同消费；yaw 以最短角差衔接，换世界时重新建立端点。
本地 limb swing 与 viewbob 行走距离在每个移动 tick 后消费实际位移，包含碰撞与 catch-up；
远程实体、披风和 bob 振幅保留各自的更新路径，乘骑时停止步行动画。
本地乘客的 tick 位置与零 tick 挂载/硬同步复用 world 的载具链附件求解；摄像机通常插值玩家
位置端点，普通可骑乘矿车启用新行为且有活动步骤时使用同一加权载具渲染姿态。
激流攻击采用独立持物姿态，自动旋转结束后恢复普通挥动；图腾激活动画保持自己的相机变换。
runtime 通过 `replace_scene_frame` 一次发布 camera 与 entity instances；提取缓存逐实例复用提交，
只重新提取变化的实例以及方向发生变化的 billboard。透明范围保留提交原点，相机移动只更新
距离和绘制顺序。独立 setter 保留给初始化与帧交接以外的更新。
`entity_models/retained.rs` 与 `retained/vertex_pose.rs` 按资源代际持有 cuboid 局部几何，部件记录持有矩阵、UV、tint、light、
overlay 与 standalone sampling 状态，并分三条路径展开：内建 shader 的纯 cuboid 批次在顶点着色器（`EntityPosePipelines`）中直接蒙皮；混合几何或资源包 shader 且不超过 `256 KiB` 的批次在 CPU 直接写出展开顶点；更大批次经 `bbb-entity-part-expansion` 计算着色器展开并复用绑定的组。
crumbling、outline、dissolve、glint 和自定义几何保留各自实际需要的输入；各 lane 的 GPU buffer
复用容量，CPU 几何与索引按变化区间上传。空层释放 mesh，资源代际、pipeline 与 draw range 独立。

天气、暗角、世界边界和末地天空的独立 RGBA 纹理统一经
`render_core::texture_sampling::create_standalone_rgba_texture` 校验并上传，使用单层
`Rgba8Unorm` 和资源指定的 blur/clamp；bind group、uniform 与生命周期仍由各机制拥有。

## 判断与处理顺序

1. runtime 在主线程消费网络事件与命令，推进 live world/input。
2. `runtime/frame.rs` 从同一 `SimulationView` 借用提取 camera、环境、实体、物品和 screen/HUD 数据。
3. renderer 在编码任何 pass 前选定 `FramePlan`，冻结目标、深度复制与最终呈现源。
4. `render` 按 world、透明合成和 presentation 的既定顺序编码，实际附件依赖决定 pass 边界。
5. world geometry、透明目标、粒子、天气、outline 和后处理沿各自阶段合成。
6. GUI 使用自己的深度与光照边界；screen blur 和裁剪保留既有提交顺序。

生命、食物、护甲和氧气共用生存/冒险模式的可见 HUD 门槛。
第一人称火焰读取本地着火计时与同步标志；二维 atlas 绘制使用最近邻采样，
不复用地形 chunk shader 的线性采样器。
Bossbar、dialog 按钮和进度 toast 在投影时消费当前语言的组件文本，宽度与换行使用解析后的内容。
Bossbar 文本随资源代次重提取，已有进度动画仅在连接改变时清理。
7. 最终 submit/present 和截图复制消费选定的 frame target。

透明合成的 `None`、`VanillaDirect` 和 `GenericGraph` 都由 `FramePlan` 选择（缺失后处理图时回退 regular 路径与 `None`，不设独立内建 combine）。
是否复制 translucent/item-entity/particle 深度，与这些目标是否有真实消费者一起决定。
不能仅依据“启用了改善透明度”给所有目标执行无条件复制。

world 的帧内几何与上传先准备，再保持原绘制顺序编码。Regular 路径复用 main color/depth pass，
只在实体轮廓换目标及其后处理取样处结束并续开；Improved 路径保留各目标的 clear、按需深度复制与合成，`VanillaDirect` 最多排序 4 个非云层并在最终混合时合入云层，无置顶调试广告牌时在同一合成 pass 直接绘制第一人称手部。
release 构建默认启用 `DISCARD_HAL_LABELS`（可通过 `WGPU_DISCARD_HAL_LABELS=0` 保留标签）；GPU 归因使用 terrain atlas animation、world、transparency combine、presentation tail 和 encoder tail
五个实际阶段，诊断标记不要求额外拆分生产绘制。
Improved 路径把世界前段的 encoder 交给工作线程执行 `finish`，主线程继续编码同一帧的后续绘制。
可以直接绘制到 surface 时，先获取当帧纹理，再编码以它为目标的 pass。截图和需要采样主画面的后处理走离屏目标。
主线程按原绘制顺序用一次 `Queue::submit` 提交当帧命令并呈现。取帧跳过或失败时收回并丢弃未提交的命令，
恢复地形动画 tick 并取消 GPU 查询。启用提交计时后，`command_encoder_finish` 累加各 encoder 的完成耗时，
`world_command_finish_wait` 记录主线程完成尾段后等候前段的时间，`queue_submit_api` 记录提交调用的耗时。

### 实体、模型层与装备

world 提供实体类型、姿态、动画和 stack facts；native 构造 `EntityModelInstance`。
`entity_models::dispatch` 和 `model_layers/` 选择具体几何，textured layer owner 决定提交。
主体、held item、装备、名称、阴影、火焰、leash 与 outline 各有独立输入，不能互相代替。
火焰保留按实际 root、尺寸、相机 yaw、天空光与 sprite UV 生成的几何；相机位置、pitch
和普通肢体动画不使它失效，atlas 像素动画仍独立推进。
动态方块实体的源投影接收 native 可见性谓词，先判断位置再解析物品、profile、图案及动画状态；
末地传送门与折跃门按类型保留不同距离，beacon 与特殊离屏边界仍由各自路径决定。
静态实体纹理只由全局 `entity_model_texture_refs()` 目录提供；测试需要的纹理子集不成为生产 API。
实体 mesh 的相机失效按实际依赖区分：billboard 跟随姿态变化，透明提交排序只跟随眼部位置变化；
原地转头只重新提取 billboard；实例更新复用其它实例的提交，纹理或资源变更使对应资源提交失效。

装备只查询同代 `ResolvedEntityEquipmentAssetCatalog`。
pack/item generation 产生 `EntityEquipmentAssetCatalog`；native prepare 解析 texture 和 atlas。
帧只携带 `EntityEquipmentAssetId`、dye、foil、trim、player texture 等事实。
未知 id、stale generation、无 layer type 或空 layer 都输出零层。

humanoid、armor stand、wings、wolf、equine、llama 和其它 mount layer 共享 catalog。
item 到 equipment asset 的映射不以“当前纹理是否准备成功”为条件。
world 的 slot admission 和 stack 存在性也不从 catalog readiness 反推。
例如 zombie nautilus BODY 非空可隐藏 corals，即使装备 asset 无法绘制。

layer emitter 保持 dye skip/default、foil、trim、baby/leggings 与 player-texture override 的顺序。
`ENTITY_MODEL_TEXTURED_SHADER` 统一处理采样、cutout、tint、overlay、lightmap、diffuse 和 fog。
实体先应用方向光，再混合受伤/闪白 overlay，最后应用 lightmap 与雾；overlay alpha 按
`OverlayTexture` 的 8 位纹理量化，选中资源包着色器时提供同样的 overlay 颜色。
单双面选择由 pipeline cull descriptor 决定，不能恢复平行 shader/cache owner。
solid 与 z-offset 变体仍从这一共享 source 派生。

### 物品模型和驻留

`bbb-client-assets::item_models` 在资源加载时将 item JSON 解为 `ItemModelDefinition`、
property、predicate 和 special model 等强类型值。renderer 的 `item_model::item_runtime`
按当前 stack/components、context、world/owner 等事实选择有序 layers，native 投影与 GPU
residency 消费这些结果。为组件精确值比较保留的原始资源树不构成另一套每帧 JSON 解释器。
加载先确定纹理索引，再从强类型定义直接烘焙运行时模型树；不保留纹理名称中间树，也不在
烘焙后重建全部分支。叶节点直接保存索引、tint、translucency 与 display transform。
固定组件类型事实归 `bbb-facts::data_component_types`；typed wire patch 和 codec 分派归
`bbb-protocol::packets::data_components` 及其 `catalog::DataComponentDescriptor`。
world 持有实际 stack、组件与预测，GUI/tooltip/model 不维护第二套组件身份或 wire schema。

GUI、ground、first person 与 entity held item 的 display context 不可混用。
顶层 resolver 先清除旧输出，再按当前 stack 的模型定义产生有序 layers。
native 的 HUD 投影只有 `HudBlockItemModel { layers, lighting, foil }`。
renderer 不再反查 item runtime、terrain model 或 pack stack 来猜测同一物品的几何。

普通物品叶子直接保留资源选择的 cuboid 几何或 generated sprite，不再按物品 ID 反查默认方块状态。
cuboid 与 generated 共用 items atlas，只形成实际存在的 solid/translucent layers；
special 和 composite child 保持各自 atlas、变换、tint 与 child order。
cuboid 的不可变几何同时持有局部 bounds 和实际几何层；资源树加载时按原始顶点及继承变换
计算精确 GROUND bounds。提取与每槽布局直接读取，不重新复制/烘焙模型或扫描 cuboid 顶点。
CPU projection 可随输入重新生成，静态 vertex/index 数据由 asset identity 驻留复用。
合并 residency frame 时，ordinary 和 special submission 共享递增的 submit order；
`block_display` 与 `item_display` 的发光状态及 `glow_color_override` 同时提交到普通几何层与特殊模型的实体轮廓目标。
GUI scissor 只在 GUI target 生效；world/entity preview target 保持自己的深度语义。

### 地形区段发布

不可变快照单独携带完整区段的均匀 block state 事实；光照与 biome 仍按自身数据读取。
固定模型路由为空的均匀区段自身构网不展开 cells；邻区实际采样沿用既有 lazy 读取。
均匀区段的 VisGraph 结果由同一 state 的 solid-render/air 事实决定，部分高度和混合区段按单元解析。
区段驻留记录拥有 compiled visibility 和首次上传淡入时间；CPU 网格缓存只持有尚待上传的几何。
网格 patch 与遮挡传播使用各自的待处理集合，光照、颜色等不改变可见连接的重建只上传网格。
遮挡全量作业分别复用区段可见性和 loaded chunk 集合的不可变输入；等待水平邻区的传播只遍历实际 frontier。
区段移除、资源重载和 level 切换在同一 owner 清理这些派生状态。
构网选择游标在主线程预算截断后跨帧续跑，世界 revision、选择或重载变化时从近处重新选择；
队列已满的目标留到后续帧。作业 identity 使用 level、mesh generation 与权威 chunk mesh revision，
区段数量从作业自身读取。chunk mesh revision 的聚合值在 world 既有失效入口更新。

地形视锥先完整包含相机所在的八格立方体，再按图发布或两度转角刷新可见列表。
淡入期间 CPU 只推进仍在淡入的区段，经 `StagingBelt` 上传 fade 与 indirect 缓冲；列表顺序与 nearby 集合未变时，renderer 仅更新 fade buffer，
复用已有绘制列表，并在同层存在长度至少为 `128` 的同页连续绘制段时使用 indirect 绘制。实体提取仍使用即时相机视锥。

### 粒子与环境

粒子 provider 输入、存活状态和 GPU billboard 是不同阶段。
`bbb-client-world::particles` 的 `ParticleEngine` 及其 `descriptors`、`instance`、`kinds` 子模块
按 `ParticleRenderGroup` 分桶保存存活实例，并在 tick 前将每组待接纳队列上限限制在 `16,384`。native `particle_runtime.rs` 准备资源，`runtime/particles.rs`
接纳 spawn，`runtime/simulation_step.rs` 推进 tick。`runtime/frame.rs` 的 `extract_particles` 对粒子做视锥剔除，并在 `world_revision` 与相机视锥未变时跨帧复用可见粒子输入及光照采样；`bbb-client-render::particles` 及其 `gpu`、`render` 子模块生成 GPU 批次。
过期实例先释放容量，新接纳实例在这一 tick 不被额外老化。
方块破坏粒子（`LevelEvent` `2001`）按事件坐标求解带位置偏移的方块形状包围盒，再发射碎屑。
粒子亮度由当前位置查询再结合 provider emission override，不能复用出生点永久亮度。

`runtime::render_extract::environment` 读取 dimension/environment attributes、效果与天气。
lightmap 消费 darkness、night vision、conduit、水下视野、boss darkening 和 sky flash 等输入。
fog、sky、cloud、weather、world border 和 vignette 有各自 renderer owner。
这些是显示派生状态，不改变服务端天气或方块光照权威值。

camera post effect 与 screen blur 分别属于世界摄像机和 screen 背景。
post-effect graph 负责 target、uniform 和执行顺序；不能将屏幕 blur 当作 camera effect fallback。
资源重载失败继续使用上一完整 generation，不能让一部分 shader 先切换。

### 投影与共享实现

同帧相机 picking 同时发布普通 reach 的原始 block hit 和最终 crosshair target，并在 `(world_revision, CameraPose)` 未变时跨帧复用拾取结果，第一人称眼部遮罩（`screen_effect_quads`）同样在 `world_revision` 未变时复用；名字牌与攻击指示读取
最终目标，方块轮廓读取原始命中并应用自己的许可规则。模拟步每步计算一次 `sky_darken`，供暗角、眼部遮罩与实体阴影共用；vignette 亮度只随 GUI tick 推进，
渲染帧读取既有平滑值，调试 HUD 计数器仅在实际显示或查询时采样。

HUD 投影只有 layers/lighting/foil，生产消费只遍历一次 model.layers。default-resource item
catalog、stone projection 与 offscreen hotbar/glint/scissor 像素共同验证这条数据流，
具体 fixture 集合以生成事实和测试为准。

装备的 slot 与 asset 来自 world 已发布的强类型 `EQUIPPABLE` 原型及 stack patch，
不再经 item registry 的独立装备表搬运；SimpleEquipmentLayer 的 SADDLE/BODY 不借用 armor-slot
classifier。humanoid/armor-stand、wings、wolf、equine、llama、pig、strider、camel、happy-ghast
和 nautilus layer 共用 resolved catalog。测试空 catalog 不启用 production fallback。

Vanilla ENTITY_SNIPPET 只在 no-cull pipeline 启用 PER_FACE_LIGHTING，entity.fsh 根据
gl_FrontFacing 选择正反法线；启用 back-face cull 时存活 fragment 恒为正面。BBB 共享 shader
保持同一边界；solid 只去掉 alpha-cutout block，z-offset 只替换 projection matrix。

## 生命周期和 fallback

资源 prepare 覆盖 catalog、owned texture、atlas 和 GPU pipeline。
完整 prepare/commit 成功才替换 generation；任何阶段失败保留旧资源与旧 projection 合同。
旧 generation 的 asset id 不会命中新 generation 的 catalog。
world/session 切换清理 level-bound scene，进程级资源寿命遵守各 owner 合同。

截图由 `bbb-client` 输入/路径 owner 发起，renderer 只完成复制与读回。
`prepare_screenshot_copy` 按 GPU 行对齐建立 buffer，并记录真实 pixel format。
`read_screenshot_pixels` 去掉行 padding、转换 BGRA/RGBA，输出紧密 RGBA8。
截图落盘与 offscreen harness 复用该读回路径；map、poll、格式或写文件失败返回错误。
不能把失败读回当成黑色的成功截图，也不能拿另一条 CPU raster 路径替代真实 frame。

## BBB 实现映射

| 语义 owner | 实际路径 | 消费者 |
| --- | --- | --- |
| native 帧提取 | `bbb_client::runtime::frame`，使用 `runtime::render_extract` 的帧值与辅助提取 | renderer typed projection |
| camera 原子替换 | `bbb_client_render::renderer::scene_state` | uniform、透明排序 |
| 帧阶段 | `bbb_client_render::render` | command encoder 和 present |
| 装备层 | `bbb_client_render::entity_models::textured::equipment_layers` | entity mesh submissions |
| 物品驻留 | `bbb_client_render::item_render::item_models::residency` | immutable asset/instance buffers |
| data-driven 实体骨骼 | `bbb_client_render::entity_models::data_driven` | `BakedDataDrivenRig` 复用 `ModelPart` / `vertex_pose.wgsl` |
| 同步无损遮挡剔除 | `bbb_client::occlusion_culling::OpaqueTerrainOccluder` | 对已完成编译且完全不透明（`fade == 1.0`、修订匹配、`solid_render`）的实体方块柱面做同帧保守遮挡判定，剔除墙后实体姿态/阴影/物品模型、普通方块实体（保留 `OffScreenBlockEntityRenderer`、`EndGateway`、发光描边、拴绳、实体光束与常显名字牌）及单面片粒子光源采样/顶点构建（模拟刻与 `E:` 存活计数及遮挡实体动态光源不变） |
| 动态光源与 horizon LOD | `bbb_client_render::render_core::camera`、`scene::lod_terrain` | `--dynamic-lights` 点光源 uniform 与 `--lod-distance` 远景地表网格 |
| 截图 | `bbb_client_render::screenshot` | PNG 和 offscreen readback |

引擎核心底座能力由启动 CLI 参数门控：`--dynamic-lights off|fast|fancy` 在 `CameraUniform` 尾部上传最多 32 个近邻点光源（手持光源、火焰/炽热原版生物及 `ContentRegistry::dynamic_light_rgb` 模组实体，其中 `fast` 在地形顶点计算而 `fancy` 在地形/实体片元计算）；`--lod-distance 16..=256` 从已加载区块高度图/顶部方块构造裙边格网并同步扩展深度远平面与雾终点；`off` / `0` 时完全跳过相应渲染开销。

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| update/extract/render 边界 | `GameRenderer` | runtime + renderer 的帧与附件边界 | `aligned` | 完整帧 GPU 验证 |
| 有序物品层 | `ItemModelResolver` | item-model + item-render residency | `aligned` | append-order 测试 |
| 装备缺失层 | `EquipmentLayerRenderer` | resolved equipment catalog | `aligned` | empty-catalog 测试 |
| 粒子 tick/admission | `ParticleEngine` | particle runtime | `aligned` | expiry/admission 测试 |

## 结构性不变量

- 一个帧的 target/copy/combine/presentation 选择只有 `FramePlan` 一个 owner。
- renderer 不反向读取 world、HTTP、磁盘 cache 或 item-model resolver。
- ordered layers、submit order、resource generation 与 target 类型不能被缓存键省略。
- 缺失装备资源输出零层；模型状态仍由独立 world facts 决定。
- screenshot 与 offscreen 使用同一真实 GPU 读回和像素格式处理。

## 聚焦测试（focused tests）和真实证据

以下用例定位行为边界；执行与会话回放见[开发流程与测试](../../development/testing-and-gates.md)。

| 测试 | 路径 | 证明的语义 |
| --- | --- | --- |
| `frame_plan_selects_feature_attachments_and_transparency_combine` | `bbb_client_render::render` | `FramePlan` 统一决定附件选择与透明合成路径 |
| `empty_item_and_particle_targets_do_not_claim_depth_work` | 同上 | 空 item/particle 目标不申请 main depth 复制 |
| `residency_frame_append_preserves_ordinary_special_submit_order` | `bbb_client_render::item_render::item_models::residency` | append 同时调整 ordinary/special 次序 |
| `resolved_empty_catalog_does_not_enable_legacy_static_equipment` | `bbb_client_render::entity_models::tests::equipment_assets` | 空 catalog 不回退 saddle 静态纹理 |
| `new_sprite_table_clears_particles` | `bbb_client_world::particles::tests` | 资源代切换清除旧粒子 |

## 当前差异

帧顺序状态不代表每个资源包 shader、每个实体姿态或每个粒子像素都由本页验收。
外部客户端没有 integrated server；依赖本地服务端的调试显示与远端实体物理信息受范围约束。
显示机制的开放项和 `not-needed` 裁决见客户端 ledger，不能用当前页面代替缺口列表。

## 账本归属

主机制：`rendering-frame-order`。
差异权威：[客户端 ledger](../../../unsupported-client-features.md) 的 renderer、entity、particle 与 resource 领域。
推进原则：[AGENTS.md](../../../../AGENTS.md)。

## 精确重开条件

`GameRenderer` 提取/呈现边界、world 绘制次序、frame target 或深度消费者变化时重核帧合同。
equipment catalog generation、item ordered layers 或粒子 tick/admission 改变时重核对应领域测试。
新增 GPU pixel format、截图 target 或资源重载提交阶段时重核读回与 rollback。

## 相关页面和源码索引

- [客户端世界生命周期](../world/client-world-lifecycle.md)：level-bound 状态的创建和清理。
- [客户端 screens、HUD 与库存](../ui/client-screens-hud-and-inventory.md)：屏幕顺序与 GUI 输入。
- [客户端聊天与身份](../security/client-chat-and-identity.md)：profile texture 信任与进程级 CPU owner。
- [离线资源信任](../resources/client-offline-asset-trust.md)：资源输入和 generation 边界。
