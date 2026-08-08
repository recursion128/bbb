# 流体渲染（fluid rendering）：表面、overlay、tint 与邻接边界

机制 ID：`rendering-fluid-mesh`

## 机制目的和边界

该机制为 water/lava 生成顶面、底面和侧面，决定高度、流向 UV、overlay、tint、光照、
透明 backface 与 block/fluid 邻接剔除。相机浸入流体与 fog 属于相机环境机制，不在此页。

## Vanilla 26.1 权威实现

`FluidRenderer.tesselate` 先读取上下与四个 cardinal 邻居的 `FluidState`/`BlockState`。
`shouldRenderFace` 要求邻居不是同类 fluid，且当前 block state 没有遮挡该面；底面另以
`0.8888889F` 检查邻居 occlusion。自身上方同 fluid 时高度为 1，否则取 own height；
非同类非 solid 邻格贡献 0，solid 贡献 -1。角高对 ≥0.8 的样本赋 10 倍权重。

顶面若零流速使用 still sprite，否则按 `FluidState.getFlow` 旋转 flowing UV，并将高度
减 `0.001F` 防 z-fighting。侧面只使用 sprite 的上半段；water 在邻居为
`HalfTransparentBlock` 或 `LeavesBlock` 时使用 overlay，overlay 不发反面，普通 water
side 发反面。每面叠加 fluid tint、方向 shade 与 current/above 最大 light。

## 决策输入与状态所有权

fluid kind、level/falling、上下/cardinal/diagonal 邻格、block face occlusion shape、
`isSolidRender`、water-overlay relation、fluid sprites/tint/light 都是独立输入。world
拥有 cell 与邻居状态；resource generation 拥有 sprites/tints；renderer 只计算 mesh。

## 判断与处理顺序

1. 读取六邻格并决定每面是否候选。
2. 算自身与四角高度；角样本保持 Vanilla 的 same-fluid/solid/empty 顺序和权重。
3. 应用 top/bottom/side offset 与 still/flow UV。
4. 侧面先决定 overlay sprite，再决定 backward face。
5. 应用 tint/light/cardinal shade，写 translucent layer 并参与排序。

## 生命周期和 fallback

chunk 或邻 chunk 的 block/fluid/light 改变使 mesh dirty。缺邻 chunk 时使用明确边界状态，
邻 chunk 到达后重建；不能将临时边界线缓存为稳定 mesh。缺 overlay sprite 走普通 side，
但不得从 block 名称猜 overlay neighbor。

## BBB 实现映射

`bbb-world` 的 `TerrainFluidState` 与 exact render facts 携带 fluid/overlay/occlusion；
`bbb-native/src/terrain_runtime/textures.rs` 投影 still/flow/overlay texture 和 tint；
`bbb-renderer/src/terrain/mesh/fluid.rs` 计算高度、流向、UV，`emitter.rs` 负责面剔除、
overlay/backface、vertex light，terrain translucent queue 负责混合与排序。

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 同类 fluid 面剔除 | `FluidRenderer.shouldRenderFace` | mesh emitter | `equivalent` | same/different-kind tests |
| own/corner height | `getHeight`, `calculateAverageHeight` | `mesh/fluid.rs` | `equivalent` | corner/above/flowing tests |
| top/side UV 与 offset | `tesselate` | fluid UV helpers | `equivalent` | UV/offset tests |
| water overlay | half-transparent/leaves query | exact `fluid_overlay_neighbor` | `equivalent` | glass/leaves adjacency tests |
| tint/light/backface | model set + level light | textures/emitter | `equivalent` | tint/light/backface tests |
| 跨 chunk 边界 | neighbor access | lookup + remesh | `equivalent` | adjacent-chunk tests + anchors |

## 结构性不变量

- water/lava 优先按 fluid type 判断；不能只按 block material。
- overlay relation 来自权威 26.1 block-state facts。
- top、side 和 overlay 的 backface 规则不得合并为统一 transparent rule。
- 透明优化后必须重跑水/玻璃、染色玻璃、叶子和 chunk-boundary ROI。

## 聚焦测试（focused tests）和真实证据

renderer tests 覆盖内部面、不同 fluid、上方同 fluid、角高、side UV、bottom offset、
backface、邻 chunk、occlusion、tint 与 waterlogged 内部面。三场景同 tick 锚点检查水
顶/侧面、玻璃交界、深度和透明排序；共同 draw evidence 证明两 API 都提交真实内容。

- `crates/bbb-native/src/terrain_runtime/textures/tests.rs#fluid_render_data_uses_still_top_and_flowing_sides`
- `crates/bbb-renderer/src/terrain/mesh/tests.rs#culls_internal_faces_between_fluid_blocks`
- `crates/bbb-renderer/src/terrain/mesh/tests.rs#keeps_faces_between_different_fluid_kinds`
- `crates/bbb-renderer/src/terrain/mesh/tests.rs#fluid_top_face_averages_corner_heights_from_adjacent_fluids`
- `crates/bbb-renderer/src/terrain/mesh/tests.rs#fluid_flowing_top_uses_flowing_texture_and_rotated_uvs`
- `crates/bbb-renderer/src/terrain/mesh/tests.rs#water_side_overlay_neighbor_uses_overlay_texture_without_reversed_back_face`
- `crates/bbb-renderer/src/terrain/mesh/tests.rs#fluid_faces_use_flat_vanilla_light_without_ambient_occlusion`
- `crates/bbb-renderer/src/terrain/mesh/tests.rs#culls_fluid_faces_between_adjacent_chunk_snapshots`
- `crates/bbb-renderer/src/terrain/mesh/tests.rs#fluid_face_occlusion_uses_vanilla_height_and_top_surface_ordering`
- `crates/bbb-renderer/src/terrain/mesh/tests.rs#waterlogged_block_emits_block_and_fluid_layers`

## 当前差异

fluid mesh 当前通过 P0 视觉合同；相机介质/fog 是相邻但独立机制，应在 parity matrix
单列，不能用本页结论替代。

## 账本/goal 路由

fluid mesh 无 active gap。相机 fog 若未关闭，状态只取
[`unsupported-features.md`](../../../unsupported-features.md)。

## 精确重开条件

目标版本、`FluidRenderer` 判断顺序或关键 water/lava ROI 变化时重开本页。

## 相关页面和源码索引

- [方块渲染](block-rendering.md)
- Vanilla：`FluidRenderer`、fluid model set、`HalfTransparentBlock`、`LeavesBlock`
