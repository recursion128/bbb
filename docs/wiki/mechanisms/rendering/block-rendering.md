# 方块渲染：剔除、遮挡、AO、光照与 layer

机制 ID：`rendering-block-model-pipeline`

## 机制目的和边界

该机制把一个 block state 的 baked quads 变为 chunk layer 顶点，并复刻 Vanilla 的面
剔除、occlusion facts、ambient occlusion（环境遮蔽）、block/sky light、cardinal shade
与 render-layer routing。fluid 另见独立页面。

## Vanilla 26.1 权威实现

- `ModelBlockRenderer.renderModel` 按 quad direction 查询相邻坐标；只有 quad 声明 cull
  时才调用私有 `shouldRenderFace`，最终委托 `Block.shouldRenderFace`。
- `Block.shouldRenderFace` 先查邻居 opposite face 的 occlusion shape；邻居完整覆盖立即
  false；再查 `state.skipRendering(neighbor,direction)`；空邻形 true，空自身形 true，
  否则 `Shapes.joinIsNotEmpty(..., ONLY_FIRST)`。
- `BlockModelLighter` 从面、四角与对角查询 `getShadeBrightness`、`isViewBlocking`、
  `getLightDampening`、`isSolidRender` 和 packed light，并按 quad geometry 权重插值。
- `SectionCompiler` 根据 quad 的 material layer 路由 buffer，另以
  `BlockState.isSolidRender` 构建 section visibility；translucent layer 生成排序状态。

## 决策输入与状态所有权

`BlockBehaviour.BlockStateBase` 缓存或提供 `canOcclude`、`isSolidRender`、
`getFaceOcclusionShape`、`isViewBlocking`、`getLightDampening`、
`getShadeBrightness` 与 `skipRendering`。查询坐标是当前 block、目标相邻 block、对应面
以及 AO 四角/对角；这些彼此独立，render layer 或方块名称不能替代。

## 判断与处理顺序

1. baked model 选择 quad 与 material layer。
2. 对 cull quad 按 Vanilla 顺序比较邻居 shape、`skipRendering` 与自身 shape。
3. 根据 quad face/geometry 选择 base/center light；采样四角和必要对角。
4. 将 AO 权重、packed block/sky light、方向 shade、tint 与 vertex 数据组合。
5. 按 layer 提交；opaque/cutout 保持 depth，translucent 进入排序/混合路径。

## 生命周期和 fallback

configuration/runtime 以 global block-state id 查生成的 26.1 facts；world chunk extraction
携带它们。缺合法 id 时不假造 occlusion 等价事实，使用显式 conservative fallback 并
暴露诊断。chunk 邻居变化使相关 mesh 失效并重建。

## BBB 实现映射

`bbb-protocol/src/block_state_types.rs::VanillaBlockStateRenderFacts` 和 support facts 是
26.1 projection；`bbb-world/src/chunks/store.rs` 按 global id 赋给 terrain cell；
`bbb-native/src/terrain_runtime.rs` 保真传入
`bbb-renderer/src/terrain.rs::TerrainCell`。剔除/AO/light/cardinal shade 在
`bbb-renderer/src/terrain/mesh/emitter.rs`，layer/透明提交在 terrain renderer。

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| face culling 顺序 | `ModelBlockRenderer`, `Block.shouldRenderFace` | terrain mesh emitter | `equivalent` | shape/skip/full-neighbor tests |
| 独立 occlusion facts | `BlockStateBase` | protocol → world → `TerrainCell` | `equivalent` | exact facts extraction test |
| AO corner sampling | `BlockModelLighter` | mesh AO sampling | `equivalent` | AO/cardinal fixture tests |
| block/sky/cardinal light | lighter + level shade | terrain vertex light | `equivalent` | full block/glowstone anchors |
| layer routing | `SectionCompiler` material layer | terrain batches | `equivalent` | cutout/translucent tests + anchors |
| translucent sorting | section sort state | renderer translucent queue | `equivalent` | static/flight/boundary anchors |

## 结构性不变量

- 不得从 `Opaque/Cutout/Translucent`、render shape 或 block name 推导独立 state facts。
- 查询相邻坐标、opposite direction 与判断顺序必须与 Vanilla 一致。
- chunk 边缘需要跨 chunk 邻居；unloaded 不能伪装成 air 后永久缓存。
- 性能优化必须重跑石材、玻璃、叶子、发光石与 chunk-boundary ROI。

## 聚焦测试（focused tests）和真实证据

`extracted_terrain_carries_exact_render_occlusion_facts` 验证 protocol facts 到 world；
renderer mesh tests 覆盖内部面、shape、AO、方向光照、邻 chunk 和透明面。共同 replay
在 `static_orbit`、`preloaded_flight`、`chunk_boundary_crossing` 保存同 tick ROI 与
RenderDoc 非空 draw evidence。

## 当前差异

当前合同的已知回归矩阵已通过；新 block state、pack model 或 renderer 优化仍需按机制
复验，不能靠全图平均误差宣称等价。

## 账本/goal 路由

当前无本机制 active gap。

## 精确重开条件

目标版本改变 block-state 表、渲染 layer 或 Vanilla 判断顺序，
或关键 ROI 回归时重开。

## 相关页面和源码索引

- [fluid rendering](fluid-rendering.md)
- [性能与验证](../../architecture/performance-and-validation.md)
- Vanilla：`BlockModelLighter`、`ModelBlockRenderer`、`Block`、`BlockBehaviour`、`SectionCompiler`
