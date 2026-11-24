# 客户端流体渲染：表面、覆盖层、着色与邻接边界

机制 ID：`rendering-fluid-mesh`

## 机制目的和边界

本机制负责为水（water）与熔岩（lava）生成顶面、底面与侧面网格，决定角点高度、流向 UV、水体覆盖层（overlay）、群系着色（tint）、打包光照、半透明背面（backface）以及方块/流体邻接面剔除。相机浸入流体介质后的视野与雾效属于相机环境机制（由 `crates/bbb-client/src/runtime/render_extract/environment.rs` 实现），不在本页网格构网范围内。

## Vanilla 26.1 权威实现

`FluidRenderer.tesselate` 首先读取上下及四个水平方向邻居的 `FluidState` 与 `BlockState`。`shouldRenderFace` 要求邻居不是同类流体，且当前方块状态未完全遮挡该面；底面另以 `0.8888889F` 高度检查邻居遮挡。上方存在同类流体时自身高度为 `1.0`，否则取当前流体高度；计算角点高度时，非同类且非实心邻格贡献 `0`，实心邻格贡献 `-1`，对 `>= 0.8` 的高度样本赋予 10 倍权重。

顶面在零流速时使用静止纹理（still sprite），流动时按 `FluidState.getFlow` 旋转流动纹理（flowing UV），并将顶面高度下移 `0.001F` 以避免深度冲突（z-fighting）。侧面仅采样纹理上半段；当水体的水平邻居为 `HalfTransparentBlock` 或 `LeavesBlock` 时改用 `water_overlay` 且不输出背面，普通水体侧面则输出背面。每个面均叠加流体着色、方向明暗系数，以及当前格与上方格的最大打包光照。

## 决策输入与状态所有权

流体种类、液位/下落标志、上下/四向/对角邻格、方块面遮挡形状、`isSolidRender`、水体覆盖层邻接关系，以及流体纹理、着色与光照均为独立输入。世界层拥有方块与邻格状态，资源代际拥有纹理与着色，渲染器仅据此计算网格。

## 判断与处理顺序

1. 读取六个相邻格状态并判定各面是否需要生成。
2. 计算自身高度与四角平均高度，保持 Vanilla 的同类流体/实心/空气采样顺序与加权规则。
3. 应用顶面/底面/侧面偏移与静止或旋转流向 UV。
4. 侧面先判定是否启用水体覆盖层纹理，再决定是否额外发射背面。
5. 叠加着色、平直打包光照与方向明暗系数，写入半透明渲染层并参与相机排序。

## 生命周期和 fallback

本区块或相邻区块的方块、流体或光照变化均会使区段网格失效并重算。相邻区块尚未加载时使用明确的边界状态，待邻区块到达后立即重绘，绝不将临时边界缝隙永久缓存。缺少覆盖层纹理时回退至普通侧面纹理，但不得依据方块名称猜测覆盖层邻接关系。

## BBB 实现映射

- `bbb-client-world`：`TerrainFluidState` 与精确方块状态渲染事实携带流体、覆盖层邻接与遮挡信息。
- `bbb_client_render::scene::terrain_textures`：投影静止、流动与覆盖层流体纹理及群系着色。
- `bbb_client_render::terrain::mesh::fluid`：计算角点高度、流向与 UV 坐标。
- `bbb_client_render::terrain::mesh::emitter` 与地形半透明队列：负责面剔除、覆盖层/背面发射、平直顶点光照以及半透明排序混合。

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 同类流体面剔除 | `FluidRenderer.shouldRenderFace` | `bbb_client_render::terrain::mesh::emitter` | `equivalent` | 同类剔除与异类保留聚焦测试 |
| 自身与四角高度计算 | `getHeight`、`calculateAverageHeight` | `bbb_client_render::terrain::mesh::fluid` | `equivalent` | 角高加权、上方同流体与流动高度测试 |
| 顶面/侧面 UV 与防冲突偏移 | `FluidRenderer.tesselate` | 流体 UV 与偏移辅助函数 | `equivalent` | 静止/流向 UV 与底面/顶面偏移测试 |
| 水体覆盖层邻接 | `HalfTransparentBlock` / `LeavesBlock` 查询 | 精确 `fluid_overlay_neighbor` 事实 | `equivalent` | 玻璃/树叶邻接覆盖层与无背面测试 |
| 着色、平直光照与背面 | 模型集 + 世界光照 | `terrain_textures` / `emitter` | `equivalent` | 着色、无 AO 平直光照与背面测试 |
| 跨区块邻接边界 | 跨区块邻居访问 | 跨区块查询与邻区到达重绘 | `equivalent` | 相邻区块快照剔除测试与回放对比 |

## 结构性不变量

- 水与熔岩判定必须优先依据流体类型，不得仅按方块材质判断。
- 水体覆盖层邻接关系必须来自权威 26.1 方块状态事实表。
- 顶面、普通侧面与覆盖层侧面的背面输出规则各不相同，不得合并为统一半透明规则。
- 涉及半透明或地形构网优化后，必须复核水体/玻璃、染色玻璃、树叶与跨区块边界 ROI。

## 聚焦测试（focused tests）和真实证据

渲染器测试覆盖同类内部面剔除、异类流体保留、上方同流体高度、四角高度平均、流向 UV 旋转、底面偏移、背面生成、跨区块快照剔除、面遮挡、着色以及含水方块双层输出；场景回放同时核对水体顶面/侧面、玻璃交界、深度与透明排序。关键用例：

- `bbb_client_render::scene::terrain_textures::tests::fluid_render_data_uses_still_top_and_flowing_sides`
- `bbb_client_render::terrain::mesh::tests::culls_internal_faces_between_fluid_blocks`
- `bbb_client_render::terrain::mesh::tests::keeps_faces_between_different_fluid_kinds`
- `bbb_client_render::terrain::mesh::tests::fluid_top_face_averages_corner_heights_from_adjacent_fluids`
- `bbb_client_render::terrain::mesh::tests::fluid_flowing_top_uses_flowing_texture_and_rotated_uvs`
- `bbb_client_render::terrain::mesh::tests::water_side_overlay_neighbor_uses_overlay_texture_without_reversed_back_face`
- `bbb_client_render::terrain::mesh::tests::fluid_faces_use_flat_vanilla_light_without_ambient_occlusion`
- `bbb_client_render::terrain::mesh::tests::culls_fluid_faces_between_adjacent_chunk_snapshots`
- `bbb_client_render::terrain::mesh::tests::fluid_face_occlusion_uses_vanilla_height_and_top_surface_ordering`
- `bbb_client_render::terrain::mesh::tests::waterlogged_block_emits_block_and_fluid_layers`

## 当前差异

流体网格生成与相机水下/熔岩/细雪雾效（见 `crates/bbb-client/src/runtime/render_extract/environment.rs`）均已落地，[客户端账本](../../../unsupported-client-features.md)当前无未关闭的 `# Todo` 缺口。两者仍属相邻但独立的机制，不能用网格测试代替相机雾效验证。

## 账本归属

流体网格机制当前无未关闭缺口；后续如发现流体渲染或相机雾效差异，统一登记于[客户端账本](../../../unsupported-client-features.md)。

## 精确重开条件

当目标协议版本、`FluidRenderer` 判断顺序或水体/熔岩关键 ROI 表现发生变化时，须重新核验本页。

## 相关页面和源码索引

- [方块渲染](client-block-rendering.md)
- Vanilla：`FluidRenderer`、流体模型集、`HalfTransparentBlock`、`LeavesBlock`
