# 客户端场景与渲染：帧提取、实体层和资源驻留

机制 ID：`rendering-frame-order`

## 机制目的和边界

本页说明客户端如何从同一世界与输入借用视图及同一资源代际提取可呈现帧，涵盖相机、实体、物品、粒子、环境光雾、后处理与截图读回。渲染器仅消费上游已决定的语义状态，既不通过绘制结果反写世界，也不运行服务端实体 AI。

本页以帧提取与绘制顺序为主线组织相邻渲染领域，不为每种实体或粒子单独切分机制页。[方块渲染](client-block-rendering.md)、[流体渲染](client-fluid-rendering.md)与[字体与文本](../ui/client-font-and-text.md)分别拥有各自的几何构网与采样细则。

## Vanilla 26.1 权威实现

源码根由 `scripts/vanilla path 26.1` 解析，以下位置均相对于该根：

| 决策 | 精确源码锚点 | 直接观察 |
| --- | --- | --- |
| 帧提取与呈现 | `net/minecraft/client/renderer/GameRenderer.java:400-471` | 先窗口与选项，再世界光照图、相机与世界场景，最后 GUI；绘制时清空深度后再进入 GUI |
| 装备层顺序 | `net/minecraft/client/renderer/entity/layers/EquipmentLayerRenderer.java:54-121` | 查询选定 `layer type` 并按声明顺序提交；首个可绘制层消费附魔光效，随后处理纹饰 |
| 顶层物品解析 | `net/minecraft/client/renderer/item/ItemModelResolver.java:38-65` | 先清空输出，再按展示上下文、世界、持有者与种子追加模型层 |
| 粒子 tick 与提取 | `net/minecraft/client/particle/ParticleEngine.java:79-148` | 先推进既有粒子组与发射器，再接纳待加入队列；按渲染组顺序提取；切换世界时清空 |

`GameRenderer.extract` 中的 `shouldRenderLevel` 同时受资源加载状态、`advanceGameTime` 与当前世界存在性约束；无世界时仍可提取 GUI，不得将菜单帧硬塞入世界渲染分支。`render` 在世界绘制之后执行实体轮廓与激活的相机后处理，再进入 GUI。源码中的自动世界截图调用点不能替代用户主动截图的独立输入与落盘契约。

## 决策输入与状态所有权

| 输入或状态 | BBB 唯一所有者 | 查询坐标与生命周期 |
| --- | --- | --- |
| 实体位置、装备与动画事实 | `bbb-client-world` | 实体 ID、世界代际、模拟 tick |
| 相机姿态、FOV、受伤抖动、晃动与旋转 | `bbb-client` | 当前观察实体与本帧 `partial_tick` |
| 物品模型解析决策 | `bbb-client-render` | 物品栈组件、展示上下文、世界/持有者/种子、资源代际 |
| 场景投影提取 | `bbb-client` 运行时 | 帧内借用的 `SimulationView` 与同代际资源 |
| GPU 资源与绘制拓扑 | `bbb-client-render` | 资源代际、帧目标与附件尺寸 |
| 物品静态几何驻留 | `bbb-client-render` | 资产标识与绘制目标类型 |

模拟 tick 与呈现帧属于两个独立时钟：同一世界状态可以连续呈现多帧，附魔光效（glint）的实时滚动不能因「世界投影未变化」而被错误跳过；相机位移也会直接影响半透明地形排序与朝向相机的实体几何。

环境采样在客户端提取侧完成。`RendererCameraFrame` 同时携带光照图、雾效、天空、姿态、受伤、行走晃动、传送门扭曲、FOV 与着色器时间。`CameraPose` 直接由 yaw/pitch 旋转构造共享视图矩阵；世界、视锥、第一人称手臂与公告牌（billboard）共用同一方向约定，垂直俯仰时保留 yaw，不通过固定 `up` 向量的 look-at 重建基底。第一人称睡眠相机按床朝向对齐水平视角并上移 `0.3`，同时覆盖本地玩家与被旁观的远端睡眠活体实体。实体模型（含本地玩家手臂）在插值位置叠加当前姿态眼高后采样天空光与方块光，避免洞口或竖井中脚部与眼部光照差异导致提前变暗；marker 盔甲架从非 marker 尺寸覆盖格选取最亮光照并传给附着装备。

`FirstPersonHandPresentation` 在模拟 tick 持有可见手持栈、抬手高度与转头惯性端点，发布后由手臂与手持物品共同消费；yaw 以最短角差衔接，换世界时重建端点。本地肢体摆动与视角晃动行走距离在每个移动 tick 后消费实际碰撞位移（含追赶 tick）；远端实体、披风与晃动振幅保留各自更新路径，骑乘时停止步行动画。本地乘客的 tick 位置与零 tick 挂载/硬同步复用世界层载具链附件求解；相机通常插值玩家位置端点，普通可骑乘矿车启用新行为且存在活动步进时使用同一加权载具渲染姿态。激流攻击采用独立持物姿态并在自动旋转结束后恢复普通挥动；不死图腾激活动画保持独立相机变换。

运行时通过 `replace_scene_frame` 原子发布相机与实体实例；提取缓存逐实例复用提交，仅重提发生变化的实例以及朝向变化的公告牌。半透明提交保留提交原点，相机移动时仅更新排序距离与绘制顺序。独立 setter 保留给初始化与帧交接以外的更新。`entity_models/retained.rs` 与 `retained/vertex_pose.rs` 按资源代际持有立方体（cuboid）局部几何，部件记录矩阵、UV、着色、光照、受击覆盖层与独立采样状态，并分三条路径展开：内置着色器的纯立方体批次直接在顶点着色器（`EntityPosePipelines`）中蒙皮；混合几何或资源包着色器且不超过 `256 KiB` 的批次在 CPU 直接写出展开顶点；更大批次经 `bbb-entity-part-expansion` 计算着色器展开并复用绑定组。破坏裂纹、实体轮廓、溶解、附魔光效与自定义几何保留各自所需输入；各通道 GPU 缓冲复用容量，CPU 几何与索引按变动区间增量上传。空层释放网格，资源代际、管线与绘制区间各自独立。

天气、暗角、世界边界与末地天空的独立 RGBA 纹理统一经 `render_core::texture_sampling::create_standalone_rgba_texture` 校验并上传，使用单层 `Rgba8Unorm` 与资源指定的模糊/钳制模式；绑定组、uniform 与生命周期仍归各机制独立拥有。

## 判断与处理顺序

1. 运行时在主线程消费网络事件与命令，推进活动世界与输入状态。
2. `runtime/frame.rs` 从同一 `SimulationView` 借用提取相机、环境、实体、物品、粒子与屏幕/HUD 数据。
3. 渲染器在编码任何 pass 前选定 `FramePlan`，冻结渲染目标、深度复制需求与最终呈现源。
4. `render` 按世界、透明合成与呈现的既定顺序编码，实际附件依赖决定 pass 边界。
5. 世界几何、半透明目标、粒子、天气、实体轮廓与后处理沿各自阶段合成。
6. GUI 使用独立深度与光照边界，屏幕模糊与裁剪保留既有提交顺序。
   - 生命、饥饿、护甲与氧气共用生存/冒险模式可见 HUD 门槛。
   - 第一人称火焰读取本地着火计时与同步标志；二维图集绘制使用最近邻采样，不复用地形区块着色器的线性采样器。
   - Boss 栏、对话框按钮与进度提示（toast）在投影时消费当前语言的组件文本，宽度与换行使用解析后内容；Boss 栏文本随资源代际重提，已有进度动画仅在连接变更时清理。
7. 最终 submit/present 与截图读回消费同一选定的帧目标。

透明合成的 `None`、`VanillaDirect` 与 `GenericGraph` 均由 `FramePlan` 选定（缺失后处理图时回落常规路径与 `None`，不设独立内置合成）。是否向半透明、物品实体或粒子目标复制主深度，严格取决于这些目标本帧是否存在真实绘制消费者，严禁仅因开启改善透明度而无条件复制全部深度附件。

世界帧内几何准备与上传先于命令编码完成，保持既有绘制顺序。常规路径复用主颜色/深度 pass，仅在切换至实体轮廓目标及其后处理采样处断开并续开；改善透明度路径保留各功能目标清空、按需深度复制与合成，`VanillaDirect` 最多排序 4 个非云层并在最终混合时合入云层，无置顶调试广告牌时在同一合成 pass 直接绘制第一人称手部。

release 构建默认启用 `DISCARD_HAL_LABELS`（可通过 `WGPU_DISCARD_HAL_LABELS=0` 保留标签）；GPU 耗时归因按「地形图集动画、世界场景、透明合成、呈现尾部、编码器尾部」五个实际阶段统计，不因诊断标记拆分生产绘制。改善透明度路径将世界前段命令编码器交给工作线程执行 `finish`，主线程并行编码同帧后续 pass。可直接绘制到窗口 surface 时先取当帧纹理再编码目标 pass；截图与需要采样主画面的后处理则走离屏目标。主线程按原始绘制顺序通过单次 `Queue::submit` 提交并呈现；取帧跳过或失败时回收并丢弃未提交命令，回退地形动画 tick 并取消 GPU 查询。启用提交计时后，`command_encoder_finish` 累加各编码器完成耗时，`world_command_finish_wait` 记录主线程收尾后等待前段的时间，`queue_submit_api` 记录提交调用耗时。

### 实体、模型层与装备

世界层提供实体类型、姿态、动画与物品栈事实；客户端构造 `EntityModelInstance`。`entity_models::dispatch` 与 `model_layers/` 选择具体模型几何，带纹理层所有者负责提交。主体、手持物品、装备、名字牌、阴影、火焰、拴绳与轮廓各有独立输入，互不替代。火焰保留按实际根节点、实体尺寸、相机 yaw、天空光与纹理 UV 生成的几何；相机位置、pitch 与常规肢体动画不使其失效，图集帧动画仍独立推进。动态方块实体的源投影接收客户端可见性谓词，先过滤位置再解析物品、profile、图案与动画；末地传送门与折跃门按类型保留不同视距，信标与离屏特例仍走各自路径。静态实体纹理只由全局 `entity_model_texture_refs()` 目录提供，测试子集不成为生产 API。实体网格的相机失效按实际依赖区分：公告牌跟随姿态变化，半透明提交排序仅跟随眼部位置变化；原地转头仅重提公告牌，实例更新复用其余实例提交，纹理或资源变更使对应资源提交失效。

装备层仅查询同代际 `ResolvedEntityEquipmentAssetCatalog`。资源包/物品代际产出 `EntityEquipmentAssetCatalog`，客户端准备阶段解析纹理与图集；帧投影仅携带 `EntityEquipmentAssetId`、染色、附魔光效、纹饰与玩家纹理等事实。未知 ID、过期代际、缺失 layer type 或空层一律输出零层。

类人生物、盔甲架、鞘翅、狼、马类、羊驼与其他坐骑层共用同一装备目录。物品到装备资产的映射不以「当前纹理是否就绪」为前提；世界层的槽位准入与物品栈存在性也不从渲染目录就绪度反推（例如僵尸鹦鹉螺 `BODY` 槽非空即可隐藏珊瑚层，即便装备资产无法绘制）。

装备层发射器严格维持染色跳过/默认值、附魔光效、纹饰、幼年/护腿与玩家纹理覆盖的顺序。`ENTITY_MODEL_TEXTURED_SHADER` 统一处理采样、镂空、着色、受击覆盖层、光照图、方向漫反射与雾效。实体先应用方向光，再混合受伤/闪白覆盖层，最后叠加光照图与雾效；覆盖层 alpha 按 `OverlayTexture` 的 8 位纹理量化，选用资源包着色器时提供等价覆盖颜色。单双面剔除由管线描述符决定，不维护平行着色器或缓存；实心与 Z 偏移变体均派生自同一共享着色器源码。

### 物品模型和驻留

`bbb-client-assets::item_models` 在资源加载期将物品 JSON 解码为 `ItemModelDefinition`、属性、谓词与特殊模型等强类型结构。渲染器 `item_model::item_runtime` 根据当前物品栈组件、展示上下文、世界与持有者事实产出有序模型层，客户端投影与 GPU 驻留消费该结果；为组件精确比较保留的原始资源树不构成每帧 JSON 解释器。加载时先确定纹理索引，再从强类型定义直接烘焙运行时模型树，不保留按纹理名索引的中间树，也不在烘焙后重建分支；叶节点直接持有纹理索引、着色、半透明标志与展示变换。固定组件类型事实归 `bbb-facts::data_component_types`，强类型线格式补丁与编解码分派归 `bbb-protocol::packets::data_components` 及其 `catalog::DataComponentDescriptor`。世界层持有实际物品栈、组件与本地预测，界面、提示框与模型解析不维护第二套组件身份或线格式定义。

GUI、地面掉落、第一人称与实体手持的展示上下文严禁混用。顶层解析器每次先清空旧输出再按当前物品栈定义生成有序层。客户端 HUD 投影只有 `HudBlockItemModel { layers, lighting, foil }`，渲染器不再反向查询物品运行时、地形模型或资源包来猜测几何。

普通物品叶节点直接持有资源选定的立方体几何或生成精灵，不再按物品 ID 反查默认方块状态。立方体与生成精灵共用物品图集，仅构造实际存在的不透明/半透明层；特殊模型与组合子节点保留各自的图集、变换、着色与子序。立方体不可变几何同时记录局部包围盒与实际几何层，加载时按原始顶点与继承变换计算精确 `GROUND` 包围盒，提取与槽位布局直接读取而无需重新烘焙或扫描顶点。CPU 投影可随输入刷新，静态顶点/索引缓冲按资产标识驻留复用。合并驻留帧时，普通提交与特殊模型提交共用递增提交序；`block_display` 与 `item_display` 的发光状态及 `glow_color_override` 同时提交到普通层与特殊模型的实体轮廓目标。GUI 裁剪（scissor）仅作用于 GUI 目标，世界与实体预览目标维持独立深度语义。

### 地形区段发布

不可变快照单独携带完整区段的均匀方块状态事实，光照与群系仍按自身数据读取。固定模型路由为空的均匀区段构网时不展开单元，邻区采样沿用既有按需读取。均匀区段的可见性图（VisGraph）直接由该状态的实心/空气事实得出，部分高度与混合区段按格解析。区段驻留记录持有编译后可见性与首次上传淡入时间，CPU 网格缓存仅保存尚待上传的几何。网格更新与遮挡传播使用独立待处理集合，仅影响光照或颜色而不改变可见连通性的重绘只上传网格。全量遮挡作业分别复用区段可见性与已加载区块集合的不可变输入，等待水平邻区的传播只遍历实际边界前沿。区段卸载、资源重载与世界切换在同一所有者清理派生状态。构网选择游标在主线程预算截断后跨帧续跑，世界修订号、选择集合或重载变化时重新从近处开始；队列满的目标留至后续帧。作业标识由世界代际、网格代际与权威区块网格修订号组成，区段数量直接取自作业本身；区块网格修订号聚合值在世界层既有失效入口更新。

地形视锥先完整包含相机所在的八格立方体，再随遮挡图发布或转角超过两度刷新可见列表。淡入期间 CPU 仅推进仍在淡入的区段，经 `StagingBelt` 上传淡入与间接绘制缓冲；可见列表顺序与近区集合未变时，渲染器仅更新淡入缓冲、复用绘制列表，并在同层存在长度至少为 `128` 的同页连续绘制段时启用 indirect 绘制。实体提取始终使用即时相机视锥。

### 粒子与环境

粒子定义输入、存活模拟状态与 GPU 公告牌属于三个分离阶段。`bbb-client-world::particles` 的 `ParticleEngine` 及其 `descriptors`、`instance`、`kinds` 子模块按 `ParticleRenderGroup` 分桶保存存活实例，并在每 tick 前将各组待接纳队列上限限制为 `16,384`。客户端 `particle_runtime` 准备粒子资源，`runtime/particles.rs` 接纳生成请求，`runtime/simulation_step.rs` 推进粒子 tick。`particle_runtime::extract::extract_particles_to_renderer`（由 `runtime/frame.rs` 调用）执行粒子视锥剔除与同步地形遮挡剔除，并在 `world_revision` 与相机视锥未变时跨帧复用可见粒子源列表及光照采样；`bbb-client-render::particles` 及其 `gpu`、`render` 子模块负责构造 GPU 批次。过期实例先回收容量，本 tick 新接纳实例不被额外老化。方块破坏粒子（`LevelEvent` `2001`）按事件坐标求解带位置偏移的方块形状包围盒后发射碎屑。粒子亮度按当前实时位置采样并结合粒子自身发光覆写，绝不永久复用出生点亮度。

`runtime::render_extract::environment` 读取维度环境属性、状态效果与天气；光照图消费黑暗、夜视、潮涌能量、水下视野、Boss 变暗与闪电天空闪白等输入。雾效、天空、云层、天气、世界边界与暗角各自拥有独立渲染所有者，均为纯显示派生状态，不改写服务端天气或世界光照权威值。

相机后处理与屏幕背景模糊分别属于世界相机与界面背景：后处理图控制渲染目标、uniform 与执行顺序，不得拿屏幕模糊替代相机后处理回退。资源重载失败继续保留上一完整代际，避免部分着色器提前切换。

### 投影与共享实现

同帧相机拾取同时发布普通触及距离的原始方块命中与最终准星目标，并在 `(world_revision, CameraPose)` 未变时跨帧复用拾取结果；第一人称眼部遮罩（`screen_effect_quads`）同样在 `world_revision` 未变时复用。名字牌与攻击指示器读取最终准星目标，方块选框读取原始命中并应用自身权限门槛。模拟步每步计算一次 `sky_darken`，供暗角、眼部遮罩与实体阴影共用；暗角亮度仅随 GUI tick 平滑推进，渲染帧直接读取当前平滑值；调试 HUD 计数器仅在显示或外部查询时采样。

HUD 物品投影仅含 `layers`/`lighting`/`foil`，生产消费单遍遍历 `model.layers`。装备槽位与资产 ID 来自世界层发布的强类型 `EQUIPPABLE` 原型及物品栈补丁，不再经由独立装备表搬运；`SimpleEquipmentLayer` 的 `SADDLE`/`BODY` 不借用护甲槽分类器。类人/盔甲架、鞘翅、狼、马类、羊驼、猪、炽足兽、骆驼、快乐恶魂与鹦鹉螺共用同一已解析目录；空目录不触发生产静态回退。

Vanilla `ENTITY_SNIPPET` 仅在无背面剔除管线启用 `PER_FACE_LIGHTING`，`entity.fsh` 按 `gl_FrontFacing` 选取正反法线；开启背面剔除时存活片元恒为正面。BBB 共享着色器维持同一边界：实心变体仅去掉 alpha-cutout 丢弃分支，Z 偏移变体仅替换投影矩阵。

## 生命周期和 fallback

资源准备覆盖目录、纹理、图集与 GPU 管线。完整准备与提交全部成功后才原子切换代际；任一环节失败则保留旧资源与旧投影契约，旧代际资产 ID 绝不命中新代际目录。世界或会话切换清空绑定当前世界的场景数据，进程级资源寿命遵守各自所有者契约。

截图由 `bbb-client` 输入与路径所有者发起，渲染器只执行纹理复制与像素读回。`prepare_screenshot_copy` 按 GPU 行对齐创建缓冲并记录真实像素格式，`read_screenshot_pixels` 去除行填充、转换 BGRA/RGBA 并输出紧密排列的 RGBA8。用户截图落盘与离屏测试共用同一读回路径；缓冲映射、轮询、格式或写盘失败一律返回错误，绝不把失败读回伪装为纯黑截图，也不用第二条 CPU 光栅化路径冒充真实帧。

## BBB 实现映射

| 语义所有者 | 实际路径 | 消费者与职责 |
| --- | --- | --- |
| 客户端帧提取 | `bbb_client::runtime::frame`（调用 `runtime::render_extract` 与 `particle_runtime::extract::extract_particles_to_renderer`） | 生成不可变强类型渲染投影 |
| 相机原子替换 | `bbb_client_render::renderer::scene_state` | 相机 uniform 与半透明排序 |
| 帧阶段与 pass 编码 | `bbb_client_render::render` | `FramePlan`、命令编码与呈现 |
| 装备层绘制 | `bbb_client_render::entity_models::textured::equipment_layers` | 实体装备网格提交 |
| 物品几何驻留 | `bbb_client_render::item_render::item_models::residency` | 不可变资产与实例缓冲复用 |
| 数据驱动实体骨骼 | `bbb_client_render::entity_models::data_driven` | `BakedDataDrivenRig` 复用 `ModelPart` / `vertex_pose.wgsl` |
| 同步无损遮挡剔除 | `bbb_client::occlusion_culling::OpaqueTerrainOccluder` | 对已编译且完全不透明（`fade == 1.0`、修订匹配、`solid_render`）的方块柱面执行同帧保守遮挡判定，剔除墙后实体姿态/阴影/物品、普通方块实体（保留 `OffScreenBlockEntityRenderer`、`EndGateway`、发光轮廓、拴绳、实体光束与常显名字牌）及单面片粒子光源采样/顶点构建（模拟 tick、`E:` 存活计数与被遮挡实体动态光源不变） |
| 动态光源与远景 LOD | `bbb_client_render::render_core::camera`、`scene::lod_terrain` | `--dynamic-lights` 点光源 uniform 与 `--lod-distance` 远景地表裙边网格 |
| 截图与像素读回 | `bbb_client_render::screenshot` | PNG 落盘与离屏 GPU 读回 |

引擎自有渲染扩展由启动 CLI 参数门控：`--dynamic-lights off|fast|fancy` 在 `CameraUniform` 尾部上传最多 32 个近邻点光源（手持光源、火焰/炽热原版生物及 `ContentRegistry::dynamic_light_rgb` 模组实体，其中 `fast` 在地形顶点计算、`fancy` 在地形/实体片元计算）；`--lod-distance 16..=256` 从已加载区块高度图/顶部方块构造远景裙边格网并同步扩展深度远平面与雾终点；设为 `off` / `0` 时完全跳过相应开销。

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 更新、提取与绘制边界 | `GameRenderer` | 客户端帧提取 + 渲染器 `FramePlan` 与附件边界 | `aligned` | 完整帧 GPU 离屏与附件路由测试 |
| 有序物品模型层 | `ItemModelResolver` | `item_model` + `item_render::item_models::residency` | `aligned` | 普通/特殊层提交序保持测试 |
| 装备缺失层处理 | `EquipmentLayerRenderer` | `ResolvedEntityEquipmentAssetCatalog` | `aligned` | 空目录不回退静态纹理测试 |
| 粒子 tick 与队列准入 | `ParticleEngine` | `bbb_client_world::particles` + `particle_runtime` | `aligned` | 粒子过期、队列上限与资源代际清理测试 |

## 结构性不变量

- 单帧的附件选择、深度复制、透明合成与呈现源只能由 `FramePlan` 唯一决策。
- 渲染器不得反向读取世界状态、HTTP 网络、磁盘缓存或物品模型解析器。
- 缓存键不得省略有序层、提交序、资源代际或绘制目标类型。
- 缺失装备资源只输出零层，实体模型逻辑状态仍由独立世界事实决定。
- 截图与离屏测试共用同一真实 GPU 读回与像素格式转换路径。

## 聚焦测试（focused tests）和真实证据

以下用例锁定关键行为边界；测试与会话回放入口见[开发流程与测试](../../development/testing-and-gates.md)：

| 测试 | 路径 | 证明的语义 |
| --- | --- | --- |
| `frame_plan_selects_feature_attachments_and_transparency_combine` | `bbb_client_render::render` | `FramePlan` 统一决定功能附件选择与透明合成路径 |
| `empty_item_and_particle_targets_do_not_claim_depth_work` | 同上 | 空物品/粒子目标不触发主深度复制 |
| `residency_frame_append_preserves_ordinary_special_submit_order` | `bbb_client_render::item_render::item_models::residency` | 追加帧时同步保持普通与特殊模型提交序 |
| `resolved_empty_catalog_does_not_enable_legacy_static_equipment` | `bbb_client_render::entity_models::tests::equipment_assets` | 空装备目录不回退旧静态鞍具纹理 |
| `new_sprite_table_clears_particles` | `bbb_client_world::particles::tests` | 资源代际切换清空旧粒子 |

## 当前差异

本页帧顺序状态不替代单个资源包着色器、具体实体姿态或具体粒子像素的专项验证。外部多人客户端不含集成服务端，依赖本地服务端的调试视图受产品范围约束。[客户端账本](../../../unsupported-client-features.md)当前无未关闭的 `# Todo` 缺口，范围裁决及后续新发现的差异均以该账本为准。

## 账本归属

主机制 ID：`rendering-frame-order`。差异权威与范围裁决见[客户端账本](../../../unsupported-client-features.md)，推进原则见 [AGENTS.md](../../../../AGENTS.md)。

## 精确重开条件

当 `GameRenderer` 提取/呈现边界、世界绘制顺序、帧目标或深度消费者变化时须重核帧契约；当装备目录代际、物品有序模型层或粒子 tick/准入规则变动时重核对应测试；新增 GPU 像素格式、截图目标或资源重载提交阶段时重核读回与回滚路径。

## 相关页面和源码索引

- [客户端世界生命周期](../world/client-world-lifecycle.md)：世界绑定状态的创建与清理。
- [客户端屏幕、HUD 与库存](../ui/client-screens-hud-and-inventory.md)：屏幕切换顺序与 GUI 输入。
- [客户端聊天与身份](../security/client-chat-and-identity.md)：profile 纹理信任与进程级 CPU 解码驻留。
- [离线资产可信链](../resources/client-offline-asset-trust.md)：资源输入与代际事务边界。
