# 客户端方块渲染：剔除、遮挡、环境遮蔽、光照与渲染层

机制 ID：`rendering-block-model-pipeline`

## 机制目的和边界

本机制负责将方块状态的烘焙四边形（baked quads）转化为区块分层顶点，复刻 Vanilla 的面剔除、遮挡事实、环境遮蔽（AO）、方块光/天空光采样、方向着色（cardinal shade）与渲染层分流。流体网格生成由[流体渲染](client-fluid-rendering.md)独立拥有。

## Vanilla 26.1 权威实现

- `ModelBlockRenderer.renderModel`：按四边形朝向查询相邻方块坐标；仅当四边形声明了 `cullface` 时才调用私有 `shouldRenderFace`，最终委托至 `Block.shouldRenderFace`。
- `Block.shouldRenderFace`：先查相邻方块对立面的遮挡形状，邻居完整覆盖时立即返回 `false`；随后检查 `state.skipRendering(neighbor, direction)`；若邻居形状为空或自身形状为空则返回 `true`，否则通过 `Shapes.joinIsNotEmpty(..., ONLY_FIRST)` 判定是否仍露面。
- `BlockModelLighter`：从当前面、四角及对角采样 `getShadeBrightness`、`isViewBlocking`、`getLightDampening`、`isSolidRender` 与打包光照，并按四边形几何权重插值。
- `SectionCompiler`：按四边形材质层将顶点分流至对应缓冲区，另依据 `BlockState.isSolidRender` 构建区段可见性图；半透明层（translucent）同步生成相机排序状态。

## 决策输入与状态所有权

`BlockBehaviour.BlockStateBase` 缓存或提供 `canOcclude`、`isSolidRender`、`getFaceOcclusionShape`、`isViewBlocking`、`getLightDampening`、`getShadeBrightness` 与 `skipRendering`。查询坐标涵盖当前方块、目标相邻方块、对应朝向以及 AO 四角/对角；各项事实彼此独立，不得以渲染层或方块名称互相推导。

## 判断与处理顺序

1. 烘焙模型选定候选四边形及其材质渲染层。
2. 对声明 `cullface` 的四边形，严格按 Vanilla 顺序比较邻居遮挡形状、`skipRendering` 与自身遮挡形状。
3. 按四边形朝向与几何形态选择基准/中心光照，并采样四角与必要对角。
4. 组合 AO 权重、打包方块/天空光照、方向明暗系数、着色（tint）与顶点属性。
5. 按材质层提交顶点：不透明（opaque）与镂空（cutout）保持深度写入，半透明（translucent）进入排序与混合路径。

`BlockState.getOffset` 的 XZ/XYZ 偏移在光照与邻居查询完成后叠加到模型顶点上，不移动方块逻辑网格或流体覆盖层。生成的渲染事实表从官方运行时逐状态提取水平与垂直偏移上限；地形构网、方块选框与碰撞形状复用同一个位置散列与偏移公式，确保花朵水平偏移、草丛下沉以及竹子/滴水石柱的水平上限与官方规则一致。

## 生命周期和 fallback

配置与运行期均以全局方块状态 ID 查询生成的 26.1 事实表，世界区块快照直接携带这些事实。遇到非法状态 ID 时绝不伪造遮挡事实，而是使用显式保守回退并暴露诊断计数。区块或邻接区块变动时失效对应区段网格并触发重建。

### 世界区段驻留与可见性

Vanilla `LevelRenderer`、`SectionRenderDispatcher.RenderSection` 与 `SectionOcclusionGraph` 共同管理区段编译、脏区标记、视锥/遮挡图遍历、透明层重排以及首次驻留淡入。BBB 沿唯一单向数据流实现该闭环：

`bbb-client-world` 区块/区段修订号 → `bbb-client::terrain_upload::TerrainUploadState` 的代际感知有界准备、驻留表与有序帧投影 → `bbb-client-render` 的 GPU 驻留、逐区段可见性 uniform 与分层绘制提交。

优先更新仅同步编译近处或玩家直接改动的脏区段，不阻塞等待其他后台网格任务；异步结果回传时校验世界与资源代际、区块修订号以及剩余脏区段数量，防止陈旧结果覆盖已同步更新的层。不透明与镂空层按区段归属稳定绘制，半透明层按可见遍历反序消费；半透明重排按区段轮转并保留每次相机更新的工作预算。遮挡图发现新可见入口时继续向外传播，确保分批编译的通道另一侧顺利进入可见集合。

客户端运行时在区段网格首次成功驻留时记录时间戳：若区段中心与相机距离平方小于 `768`，或区段从先前为空变为非空，则淡入时长为 0；其余区段使用默认 750 ms，并按 Vanilla 的整数毫秒进度计算 `ChunkVisibility`。重编译保留首次驻留时间而不重启淡入；世界替换、资源重载清空或区段卸载同时清除驻留与投影。普通方块实体与世界加载就绪度共享同一投影并以 `visibility >= 0.3` 放行；实体可见性查询 `is_entity_section_compiled_and_visible`（当前遮挡遍历已访问且淡入 `>= 0.3`，或已访问的驻留空区段）。

渲染器不反向读取世界或资源包，也不维护第二份可见性真值。不透明、镂空与半透明绘制共用同一个紧凑的逐区段 uniform；内置着色器与选中的 Vanilla `terrain.fsh` 均先用 `ChunkVisibility` 在雾色与地形色间插值，再执行距离雾效。破坏裂纹保持常量可见度，不受地形区段淡入干扰。新驻留网格在收到代际匹配的可见投影前不会自动显示；过期准备结果或过期投影在写入驻留前被直接丢弃，重载失败继续沿用上一完整代际。

### 世界场景投影与方块选框

Vanilla `LevelRenderer` 的场景提取与提交路径（可见实体、方块实体、方块破坏裂纹、方块选框与实体轮廓）在 BBB 中汇聚为一条单向链路：

`bbb-client-world` 实体/方块/破坏真值 → 客户端同帧不可变投影 → 渲染器私有 GPU 网格、轮廓目标与有序 pass 提交。

方块选框形状由 `bbb-client-world::block_outline` 裁决；当前资源代际的方块状态模型目录枚举加权变体的全部候选以及 multipart 的所有选择器，以候选材质标志的并集决定提交阶段：纯不透明选框在半透明地形前提交，包含半透明材质的选框在半透明地形后提交。普通轮廓采用黑色 alpha `102`、线宽 `max(2.5, surfaceWidth / 1920 * 2.5)`；高对比度轮廓先绘制 7 px 不透明黑色底线，再提交 `0xff57ffe1` 主线。世界边界、旁观/冒险模式门槛在投影前判定；方块破坏裂纹只投影方块中心与相机眼部距离平方不超过 `1024` 的条目。

资源包层仅发布不可变 CPU 模型目录，客户端层仅作代际匹配的材质/形状/场景决策，渲染器仅维护 GPU 驻留与 pass 排布，三层之间不存在反向读取或第二套选框解析器。资源重载在渲染器事务内准备地形目录，所有可失败模块全部成功后才原子发布新代际，失败则保留上一完整代际。

### 世界帧图与透明渲染路径

Vanilla `LevelRenderer.renderLevel` 的多 pass 编排、功能目标、透明合成链与云层路由在 BBB 中由单一决策值驱动：

`--improved-transparency` + 全景相机标志 → 客户端不可变 `LevelFrameRoute` 投影 → 渲染器私有目标、深度复制、透明合成与最终呈现路由。

默认 FANCY 走常规（regular）路径；即使启用改善透明度，全景相机也强制回落常规路径。常规路径将半透明地形、物品实体、粒子、云层与天气依次加载到主颜色/深度缓冲，不复制主深度，也不运行透明合成。改善透明度（improved）路径为五类目标分配独立颜色与深度附件，`copy_main_depth_to_feature_targets` 将主深度复制到 `translucent`，并仅当 `itemEntities` 或 `particles` 存在实际深度绘制时才复制到对应目标；云层与天气各自清空独立深度。缺失后处理图时自动回落常规路径（`improved_transparency = false`、`FrameTransparencyCombine::None`）；`VanillaDirect` 最多排序 4 个非云层并在最终混合时合入云层，无置顶调试广告牌时在同一合成 pass 直接绘制第一人称手部，呈现阶段始终读取同一路由决策。

两条路径均保持「clear → 天空/主场景 → 实体轮廓目标 → 半透明/物品/粒子 → 实体轮廓后处理 → 云层 → 天气 → 可选透明合成 → 晚阶段调试」顺序；晚阶段调试清空主深度，实体轮廓链不受改善透明度开关影响。资源与后处理代际沿用既有事务提交，失败保留上一完整代际，帧路由不缓存也不改写资源包。

### 世界效果 tick 与方块破坏生命周期

Vanilla `GameRenderer.tick` 仅在世界 tick 正常推进时调用 `LevelRenderer.tick`，后者依次推进渲染 tick、发射雨滴粒子与雨声，并每 20 tick 清理一次存活超过 400 tick 的方块破坏进度。`destroyBlockProgress` 按破坏者 ID 更新或移除条目，同一坐标显示按 `(progress, id)` 排序后的最高阶段。

BBB 对应的单向数据流为：网络包/本地交互 → `bbb-client-world` 权威破坏状态与 `block_destruction_render_stages` 有序投影 → 客户端裂纹覆盖层提取 → 渲染器裂纹 pass。服务端与本地破坏阶段只在世界层合并，客户端不再维护独立解析缓存。`WeatherEffectRuntime` 唯一持有跨世界切换与资源重载的天气 tick 与雨声游标；客户端先提交天气效果，再执行世界层过期清理。暂停或冻结 tick 时不推进，服务端步进时恢复推进。世界切换清除破坏条目与本地阶段，但与 Vanilla 长期存活的 `LevelRenderer` 一样保留天气游标。

本地破坏阶段同样只有一条上游链路：世界层 `LocalPlayerInteractionState` 按挖掘配置与手持物品签名计算进度，客户端在成功预留对应 `START`/`ABORT`/`STOP` 网络命令 permit 后才允许发布本地预测或清空目标。队列准入失败保持旧阶段；服务端方块更新与确认包校正同一预测记录。首次开始与创造模式持续破坏检查世界边界，生存模式持续破坏同一目标不重复检查；目标变为空气时清零阶段且本 tick 不触发击打粒子与挥手。本地写入空气前同时校验 `Item.canDestroyBlock`（`DebugStickItem` 拒绝、工具创造可破坏标志以及生成目录中的 `GameMasterBlock` 成员）；拒绝本地销毁不取消出站协议命令。

网络新增破坏者 ID 的驻留上限为 4,096；达上限后已有 ID 仍可更新或移除，新增 ID 被拒绝并计入 `block_destructions_ignored`，`block_destructions_tracked` 实时反映当前驻留数。裂纹投影仅分配本帧不可变切片，缺裂纹纹理时输出零覆盖层。

### 世界诊断与调试几何收尾

Vanilla `GameRenderer.extract` 先完成 `LevelRenderer.extractLevel` 再提取 GUI，因此 `getEntityStatistics` 读取的是本帧已提取的可见实体数、世界实体总数与服务端模拟距离。BBB 在唯一实体可见性缓冲完成本帧存活筛选后才构造 `E: visible/total, SD: distance`；可见数绝不从模型实例数反推（单个实体可产出多个模型部件）。简化调试信息下自定义条目仍可展示，未收到模拟距离时显示 0。

所有外部多人可达的调试几何经过同一个帧内所有者：世界/输入/资源事实 → 客户端 `DebugRenderExtractionBuffers` → 不可变 `RendererFrame` → 渲染器调试 pass。普通包围盒/矩形/文字/线段在半透明特性之后、半透明地形之前收尾；当前 `setAlwaysOnTop` 客户端调用均为文字，在合成后清深度的晚阶段 pass 提交。空帧以替换清除旧几何，GameTest 标记按世界代际与修订号替换，远端大脑调试选择按世界代际清除。

Vanilla 26.1 客户端没有 `persistForMillis` 或 `fadeOut` 的生产调用，本地集成服务端专属的逐 tick 调试收集器不在外部客户端范围内。BBB 不另建第二套长期调试收集器；调试订阅包继续受 2 MiB 载荷与 65,536 集合条目上限约束，渲染器不反向读取数据包、世界或资源包。

### 世界打包光照查询

Vanilla `getLightCoords(level, pos)` 先取 `pos` 处方块状态，再进入显式重载 `getLightCoords(brightnessGetter, level, state, pos)`：先判断 `state.emissiveRendering(level, pos)`，命中直接返回方块光 15、天空光 15；否则亮度查询器按「天空光 → 方块光」读取两层光照并打包，最后仅将方块光通道提升至至少 `state.getLightEmission()`。显式重载的状态参数与查询坐标严禁合并：方块面偏移采样在邻居坐标读取原始亮度，却继续沿用被渲染方块的状态；流体与 AO 的相邻查询则使用各自位置的状态事实。

BBB 从统一的 29,873 状态运行时报告生成 `light_emission` 与 `emissive_rendering`，由 `bbb-render-types::VanillaLightCoords` 唯一执行钳制、状态覆盖、分量取最大值、打包与归一化。`bbb-client-world::chunks::store::sample_render_light_for_state` 仅组合显式状态、`BlockPos` 与原始光照存储；默认查询再从该坐标读取状态。地形快照只携带两项固定状态事实，客户端方块实体、画作、粒子、天气及渲染器地形/流体全部消费同一纯值投影，不再各自打包或按方块名、材质猜测发光属性。

粒子在区块缺失时严格使用方块光 0 / 天空光 15；画作、告示牌与天气保留各自的缺失语义。玩法逻辑、洞穴氛围、暗角与光照调试仍读取原始光照，不受渲染光照覆写影响。资源包模型元素的 `light_emission` 仅作用于该元素本身，不能替代方块状态发光。状态事实随协议版本固定；地形网格只接受不可变快照，世界/资源换代或陈旧后台结果不能部分覆盖上一驻留代际。

## BBB 实现映射

`bbb_facts::block_state_types::VanillaBlockStateRenderFacts` 与支撑事实表承载 26.1 静态事实；`bbb_client_world::chunks::store` 按区段冻结调色板、群系与光照层。`terrain::snapshot` 复用区段修订号未变的输入，构网首次读取时才展开区段并由邻区任务共享，主线程不逐格展开整列。均匀方块、群系与光照的区段仅保留单一渲染单元；光照输入复用 `LightDataLayer::Uniform`，本地重光照也不为均匀层分配数组。本地编辑按实际依赖传播光照：仅发光变化只重算方块光并准备受影响区段及上下邻带；天空遮挡变化才导入 3×3 列的完整天空层并重算光源，不以高度裁剪跳过天空光源。垂直范围保留非空邻区及上下采样带，已加载的空区段独立保持驻留状态。

逐状态事实由 `TerrainBlockRenderContext` 按全局状态 ID 查询；`bbb_client_render::terrain::TerrainBlockRenderTable` 在每个资源代际仅将每个状态 ID 解析一次（模型、着色槽、遮挡/流体/跳过事实），供 Rayon 线程池上的构网任务只读共享。面剔除、AO、光照与方向着色位于 `bbb_client_render::terrain::mesh::emitter`，单遍按渲染层分流并直接写入 GPU 顶点；半透明提交在地形渲染器完成，`multi_draw_indexed_indirect` 经 `StagingBelt` 上传，仅在同层存在长度至少为 `128`（`MIN_INDIRECT_DRAW_RUN`）的同页连续绘制段时启用。

结果接收、合并、快照提取与任务提交共用主线程工作预算；按完成顺序接收构网结果，避免队首慢任务阻塞后续区段。候选排序与驻留集合仅依赖已加载区块集合与缓存中心；有序候选覆盖全部已加载列，快照保留、网格驻留与邻域查询共用同一集合。内容修订号仅驱动脏区段构网，不重扫整窗。主线程单遍近到远遍历候选，将脏区段直接送入任务队列；玩家优先同步编译与剩余异步编译保持各自策略，资源与世界换代时统一清空输入。

### 落沙粒子（FallingDust）静态事实边界

`FallingDustParticle.Provider.createParticle` 的无上下文静态规则同样纳入版本化方块状态事实，不再由客户端按方块名称或属性字符串重建。运行时提取报告逐状态记录：非空气不可见拒绝标记、`FallingBlock.getDustColor` 可选 RGB，以及仅当 `BlockColors.getTintSource(state, 0)` 缺失时才启用的可选地图缓存 RGB。

`bbb-facts::block_state_types::vanilla_falling_dust_static_facts` 是唯一公开查询入口，仅接受合法范围内的 26.1 全局状态 ID，非法 ID 返回 `None`。客户端依次选用落沙覆写色、当前资源代际的动态/群系着色，最后才使用静态地图色回退，仅将最终 RGBA 提交给渲染器。静态事实随协议版本固定；动态着色目录随资源代际原子替换，准备失败保留上一完整代际。

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 面剔除判断顺序 | `ModelBlockRenderer`、`Block.shouldRenderFace` | `bbb_client_render::terrain::mesh::emitter` | `equivalent` | 遮挡形状、`skipRendering` 与满格邻居测试 |
| 独立方块遮挡事实 | `BlockStateBase` | `bbb-facts` → 世界快照 → `TerrainBlockRenderTable` | `equivalent` | 精确渲染与遮挡事实提取测试 |
| 环境遮蔽（AO）四角采样 | `BlockModelLighter` | 地形构网 AO 采样 | `equivalent` | AO 对角遮挡与方向着色测试 |
| 打包方块/天空光查询 | `LevelRenderer.getLightCoords` / `BrightnessGetter` | 生成事实表 → 世界显式状态查询 → `VanillaLightCoords` | `equivalent` | 状态/坐标分离、发光覆写、缺失回退与防回流测试 |
| AO、方向着色与元素自发光 | `BlockModelLighter` + 世界着色 | 地形顶点光照计算 | `equivalent` | 平直/AO/流体光照、元素自发光与六面方向着色测试 |
| 材质渲染层分流 | `SectionCompiler` 材质层 | 地形分层批次 | `equivalent` | 不透明/镂空/半透明分流与单遍等价输出测试 |
| 半透明层排序 | 区段排序状态 | 渲染器半透明绘制队列 | `equivalent` | 静态/飞行/跨区块边界排序与离屏回归测试 |
| 区段编译、剔除与淡入周期 | `LevelRenderer`、`RenderSection`、`SectionOcclusionGraph` | 客户端有序区段投影 → 渲染器逐区段 uniform | `equivalent` | 淡入阈值、重编译、代际失效、选中地形着色器与像素测试 |
| 后端、重载与尺寸调整周期 | `LevelRenderer` 生命周期方法 | 世界修订号 → 客户端代际事务 → 渲染器私有 GPU 后端 | `equivalent` | 原子 resize/回滚、RAII 释放与资源/世界换代测试 |
| 世界场景提取与提交 | `LevelRenderer` 场景提取链路 | 世界真值 → 客户端不可变帧投影 → 渲染器 pass/目标 | `equivalent` | 选框阶段/样式、32 格裂纹距离与离屏像素测试 |
| 世界帧图与透明路由 | `LevelRenderer` 帧图与后处理链 | CLI/客户端 `LevelFrameRoute` → 渲染器目标/pass | `equivalent` | 常规/改善透明像素读回、计数器与后处理图回滚测试 |
| 世界效果与破坏生命周期 | `LevelRenderer` 效果与破坏进度 | 世界破坏状态 + 客户端天气 tick → 不可变覆盖层 | `equivalent` | 20/400 清理、冻结、拆世、4096 预算与顺序测试 |
| 世界诊断与调试几何 | `LevelRenderer` 实体统计与调试几何 | 客户端本帧可见性/调试投影 → 常规/晚阶段 pass | `equivalent` | 可见/总数统计、代际失效、空帧清除与 pass 顺序测试 |
| 落沙粒子静态颜色事实 | `FallingDustParticle.Provider` | `bbb-facts` 生成事实 → 客户端着色决策 | `equivalent` | 生成器新鲜度校验与全状态族单元测试 |

## 结构性不变量

- 严禁从不透明/镂空/半透明层、渲染形状或方块名称反推独立方块状态事实。
- 相邻坐标偏移、对立朝向与面剔除判断顺序必须与 Vanilla 一致。
- 打包光照的状态参数、亮度查询坐标与模型元素自发光必须严格分离；生产消费者不得恢复私有的原始光照打包或钳制逻辑。
- 区块边缘必须查询跨区块邻居，未加载区块不得伪装为空气并永久缓存。
- 区段淡入、实体/方块实体可见性与加载就绪门槛必须读取同一代际匹配投影，禁止建立独立二值可见缓存。
- 世界层只发布修订号，客户端只拥有地形/资源事务，GPU 表面、渲染目标、后处理图、采样器与管线完全由渲染器后端独占；下游不得反向读取世界或资源包，也不得另设漂移代际。
- 窗口尺寸调整必须先完整准备全部尺寸相关对象再一次性原子替换主缓冲、深度、透明、轮廓、云层、模糊与后处理目标；任一步失败保留上一完整代际，零尺寸不进入准备。资源释放统一走渲染器 RAII。
- 方块选框提交阶段必须由当前代际完整模型候选材质并集决定，不得按方块名、单候选或独立资源解析器判断，也不得在两个阶段重复提交。
- `LevelFrameRoute` 是功能附件、深度复制、透明合成与呈现源的唯一决策者，不得以「后处理目录存在」替代选项与全景相机判定。
- 方块破坏最高阶段只能由世界层投影合并，客户端与渲染器不得重建合并逻辑、进度缓存或独立过期游标；同 tick 中天气效果必须先于破坏过期清理及后续实体效果。
- 性能优化改动必须复核石材、玻璃、树叶、萤石与跨区块边界 ROI。
- 落沙粒子不得恢复按方块名称/属性字符串匹配颜色；动态着色必须先于可选静态地图色回退。

## 聚焦测试（focused tests）和真实证据

`extracted_terrain_carries_exact_render_occlusion_facts` 验证 `bbb-facts` 静态事实准确传递至世界层；渲染器网格测试覆盖内部面、形状、AO、方向光照、跨区块边界与半透明层。完整场景对比通过 `scripts/client_replay.py` 与[性能回归对照](../../development/testing-and-gates.md#性能回归对照)验证同 tick ROI 与真实 GPU 绘制输出。

核心行为聚焦测试包括：

- `bbb_client_world::chunks::tests::terrain::extracted_terrain_carries_exact_render_occlusion_facts`：锁定石材、玻璃与树叶的实体、遮挡、挡光、减光、着色与面形状事实。
- `bbb_client_world::chunks::tests::terrain::terrain_skip_rendering_uses_current_session_bars_tag_membership`：锁定 `skipRendering` 使用当前会话标签成员，禁止按名称或固定序号替代。
- `bbb_client_render::terrain::mesh::tests::box_model_culls_only_faces_marked_by_cullface`：验证满格邻居与部分自身形状的非对称面剔除结果。
- `bbb_client_render::terrain::mesh::tests::face_culling_consumes_vanilla_face_shape_palette_not_render_cuboids`：验证空/满/部分面形状调色板的 `ONLY_FIRST` 方向判定。
- `bbb_client_render::terrain::mesh::tests::cross_chunk_slab_neighbor_keeps_partial_face`：验证跨区块台阶邻居不误剪部分露面。
- `bbb_client_render::terrain::mesh::tests::ambient_occlusion_queries_corner_blockers_beyond_side_brightness_samples`：验证 AO 对角遮挡与侧边亮度采样为独立输入。
- `bbb_client_render::terrain::mesh::tests::terrain_diffuse_vertices_carry_vanilla_default_cardinal_shade`：锁定六面默认方向明暗系数。
- `bbb_client_render::terrain::mesh::tests::layers_split_opaque_cutout_and_translucent_faces`：验证不透明、镂空与半透明仅按材质层分流。
- `bbb_client_render::terrain::mesh::tests::exact_output::section_meshes_match_the_pre_rewrite_output_exactly`：锁定单遍分流后各层顶点与索引字节与三遍实现完全一致。

其余子机制的聚焦测试分别覆盖：

- **区段生命周期**：客户端近距/由空变非空、远距 `0 → 0.3 → 1` 淡入、重编译、换世、陈旧代际及实体/方块实体/就绪度可见阈值测试；渲染器 uniform 布局、半透明重排、离屏像素淡入与选中 Vanilla `terrain.fsh` 编译/覆盖/回滚测试。
- **后端生命周期**：渲染器 resize 计划的成功、零尺寸与分阶段故障注入测试（准备失败不改旧目标与计数，提交原子更新），以及换世、全量重载、陈旧网格、资源重载成败、轮廓/云层与图集采样器测试。
- **场景投影**：客户端实体/乘客/相机、方块实体淡入/全局/裂纹、世界边界、高对比度线宽、同代际材质阶段与 32 格裂纹测试；渲染器真实 GPU 离屏测试验证不透明/半透明选框阶段与青色高对比度像素。
- **帧图与透明路由**：CLI 默认/显式参数、全景相机抑制、单一投影、常规/改善透明附件/深度复制/合成、后处理图回滚及离屏整帧像素读回测试。
- **效果生命周期**：世界层 ID/坐标更新、非法移除、服务端/本地最高阶段合并、4,096 上限、20/400 过期、换世游标测试，以及客户端确定性天气、冻结/步进、执行顺序、裂纹与缺纹理回退测试。
- **诊断与调试几何**：当前帧可见/总数统计、GameTest/远端代际清理、空帧清除及标准/置顶 pass 深度测试。
- **光照查询**：生成器非法值与新鲜度校验、`bbb-facts` 典型方块状态、世界显式状态/坐标/原始光分离、渲染器平直/AO/流体/状态发光，以及客户端方块实体、画作、粒子天空光回退、天气与陈旧网格测试。
- **落沙粒子事实**：生成器校验、`bbb-facts` 全调色板代表状态断言，以及客户端 provider 拒绝、落沙覆写→动态着色→静态地图色回退与随机序列测试。

涉及渲染路径或视觉语义变更时，配合 `scripts/client_replay.py` 复跑石材、萤石、树叶与玻璃 ROI 回放对比。

## 当前差异

[客户端账本](../../../unsupported-client-features.md)当前无未关闭的 `# Todo` 缺口。新增方块状态、资源包模型、区段可见性逻辑或渲染器优化后，仍须按对应机制与 ROI 复验，不能仅凭全图平均误差宣称等价。

## 账本归属

本机制的未关闭差异与范围裁决统一由[客户端账本](../../../unsupported-client-features.md)维护。

## 精确重开条件

当目标版本变更方块状态表、渲染层规则或 Vanilla 剔除/光照判断顺序，或关键场景 ROI 出现回归时，须重新核验本页。

## 相关页面和源码索引

- [流体渲染](client-fluid-rendering.md)
- Vanilla：`BlockModelLighter`、`ModelBlockRenderer`、`Block`、`BlockBehaviour`、`SectionCompiler`
