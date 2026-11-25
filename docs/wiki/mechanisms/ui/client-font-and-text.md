# 客户端字体与文本：字体提供者、字形度量与图集

机制 ID：`ui-font-provider-pipeline`

## 机制目的和边界

本机制从资源包字体 JSON 解析 `bitmap`、`space`、`reference`、`unihex`、`ttf` 字体提供者（provider）与 `filter` 过滤器，按 Vanilla 提供者优先级生成字形度量、位图与共享图集，供 HUD、屏幕、地图与世界实体文本统一使用。Unicode 双向重排（bidi）/塑形、旧版 `§` 格式码调用面、组件阴影、装饰线局部索引、聊天页布局与二维字形点击几何均共享同一套字形度量。
聊天输入编辑、命令语法着色、UTF-16 `JavaString` 编解码与 SNBT/命令 Packrat 语法归[聊天编辑、命令语法与 UTF-16 文本](client-chat-and-command-text.md)；ICU 区域与时区日期格式化（`bbb-client-world::local_time`）归[客户端世界生命周期](../world/client-world-lifecycle.md)。

## Vanilla 26.1 权威实现

- **`FontManager` 与 `FontSet`**：收集字体构建器并解析 `reference` 依赖，合并外层与被引用 `filter`（外层同名覆盖内层）；单份字体 JSON 编解码失败丢弃该文件，缺失或循环引用使所属字体整体失效。`FontSet.selectProviders` 按 `FontOption.Filter` 筛选并仅保留至少首供一个码点的活跃提供者，`computeGlyphInfo` 按提供者顺序取首个字形，并为聊天输入框另选 `nonFishy` 字形。
- **`UnihexProvider`（`UnihexProvider.java:436-448`）与 `BitmapProvider`（`BitmapProvider.java:55-73`）**：`unihex` 解析 ZIP 中 `.hex` 条目的 4/5/6 位码点与 8/16/24/32 px 宽位图（同 ZIP 后读覆盖先读），先按声明序应用 `size_overrides`，其余自动扫描非零列，固定高 16、`oversample = 2`、`advance = width / 2 + 1`。缺字图由 `SpecialGlyphs.java:14-20` 与 `FontTexture.java:43-51` 定义为 5×8（`advance = 6`、`ascent = 7`、`0.01` px UV 内缩）。
- **`TrueTypeGlyphProviderDefinition` / `TrueTypeGlyphProvider`**：`file` 加载时添加 `font/` 前缀，编解码字段为 `size`（默认 11）、`oversample`（默认 1）、`shift`（默认 `[0, 0]`，每维 `[-512, 512]`）与 `skip`（字符串或列表）。按 `round(size * oversample)` 缩放轮廓，`shift` 仅平移轮廓与 bearing 而不改变 `advance`，所有度量除以 `oversample` 产出浮点 `bearing_left` / `bearing_top`；零面积位图交由 `EmptyGlyph`，8 位灰度覆盖率写入共享图集。
- **组件样式、地图与聊天页**：阴影颜色转换遵循 `ExtraCodecs.java:140` / `ExtraCodecs.java:222-225` 与 `Style.applyTo`；地图名称与背景管线对应 `MapRenderer.render`、`RenderPipelines.java:80-84`、`BlendFunction.java:10-11`；聊天页布局、队列提示、滚动条颜色与二维点击对应 `ChatComponent.java:52-54`、`ChatComponent.java:225`、`ActiveTextCollector` 与 `PlayerEntry.java:61-65`、`Options.java:660`。

### 组件文本的旧版 `§` 格式码调用面

`StringDecomposer.iterateFormatted` 是唯一将 `§` 解析为格式码的入口（`§` 与后继单元成对消费，未知码消费不改样，末尾孤立 `§` 终止；颜色码清除 5 种装饰标志并保留字体/事件/阴影，`§r` 恢复传入的 `resetStyle`）。组件文本在视觉顺序边界（`MutableComponent.getVisualOrderText` → `SubStringSource.create`）完成解析，已解析的 `FormattedCharSequence` 与 `EditBox` 原始值（`FormattedCharSequence.forward`、`plainIndexAtWidth`）不再二次解析：

| Vanilla 调用面 | 关键方法 | `§` 解析规则与复位样式 |
| --- | --- | --- |
| 组件视觉顺序、宽度与分行 | `SubStringSource.create`、`StringSplitter.{stringWidth,headByWidth,splitLines}` | 按片段继承样式复位；`splitLines` 测量用 `initialStyle`，切出片段再于视觉顺序阶段解析 |
| 纯字符串绘制与补全/调试文本 | `Font.prepareText(String)`、`CommandSuggestions`、`AbstractSignEditScreen`、F3 / 统计 / 游戏模式切换 | `Style.EMPTY` 或调用方传入样式复位（如社交玩家状态 `PlayerEntry.java:61-65` 传入 `ITALIC`） |
| 聊天 HUD、书本、告示牌与提示框 | `ChatComponent`、`BookViewScreen.visitText`、`AbstractSignRenderer`、`ClientTextTooltip` | 按 `chatColors` 剥离或解析后驱动行宽、截断与绘制 |
| 对话框、动作栏/标题、世界文本与侧边栏 | `DialogBodyHandlers`、`Gui`、`EntityRenderer`、`DisplayRenderer`、`MapRenderer`、`PlayerTabOverlay`、`BossHealthOverlay`、`EffectsInInventory` | 先解析一次 run 列表，再共用同一宽度、分行、居中/右对齐、点击热区与绘制 |
| 原始可编辑输入值（**不解析 `§`**） | `EditBox.applyFormat`、`StringSplitter.plainIndexAtWidth` | 聊天/命令输入行、书页编辑值、社交搜索框输入值与容器输入框原样显示 `§` |

## 决策输入与状态所有权

| 输入/状态 | 唯一所有者 | 下游消费 |
| --- | --- | --- |
| 字体 JSON/ZIP/PNG/TTF 字节、优先级、过滤器与安全预算 | `bbb_client_assets::PackResourceStack` | 字体提供者加载与代际事务 |
| 提供者依赖图、字形映射（普通 + `nonFishy` + 混淆桶）与共享 RGBA 图集 | `bbb_client_render::item_model::font`（`providers`、`unihex`、`ttf`、`missing`） | `HudFontAtlasImage` / `HudFontGlyphBook` |
| 双向重排、装饰线局部索引与纯字形度量原语 | `bbb_render_types::{hud_glyphs, text_visual_order}` | HUD 与世界文本排版共用 |
| 组件模板分解、旧版 `§` 扫描与事件身份 ID | `bbb_client::component_text`、`WorldTextEventIds` | 屏幕、HUD、地图、告示牌、TextDisplay 与实体名 |
| 聊天页几何布局、滚动状态与二维字形点击热区 | `bbb_client_render::hud::text_layout::chat_geometry`、`bbb_client::input::HudChatScroll` | 渲染绘制、滚轮/PageUp/PageDown 与点击命中 |

## 判断与处理顺序

### 1. 提供者解析、字形选择与纯 Rust TTF 光栅化

1. **资源栈与依赖解析**：同名字体按资源栈从高到低收集提供者，单文件编解码错误整文件跳过；先展开 `reference` 并合并 `filter`（外层覆盖内层），缺失或循环引用令该字体整体失效。被过滤器隐藏的提供者仍支付加载检查与内存预算。
2. **码点目录与首供优先**：`bitmap`、`space`、`unihex` 均使用 `u32` 全码点键（`0..=0x10FFFF`，含 `D800..=DFFF` 代理码点并纳入计费与混淆桶）；仅保留至少首供一个码点的活跃提供者。普通文本取首供字形；聊天输入框使用 `nonFishy` 字形（首个 normal/bold `advance` 均在 `[0, 32]` 内的活跃后备，无合格者回退缺字）；未知字体 ID 使用缺字字体，不借用默认字体。
3. **纯 Rust `ab_glyph` TTF 管线**：桌面与 `wasm32` 统一在 `crates/bbb-client-render/src/item_model/font/ttf.rs` 使用 `ab_glyph` 解析轮廓与惰性光栅化（零 FreeType 动态库依赖）：
   - 校验 SFNT 魔数、`glyf` 表、Unicode `cmap` 与 `MAX_TTF_ASSET_BYTES = 16 MiB`；
   - 参数安全钳制：`oversample >= MIN_TTF_OVERSAMPLE (0.1)`、像素字号 `round(size * oversample).max(1)`、`shift` 按 1/64 像素取整、`advance` 钳至 `[-4096.0, 4096.0]`、单字形上限 `TTF_PAGE_MAX_DIMENSION = 8_192`、码点上限 `MAX_TTF_GLYPHS = 1_114_112`；
   - 加载期仅保留原始字体字节与度量目录（按 `glyphs.len() * 96 + bytes.len()` 计入派生预算），绘制时按需光栅化单字形 8 位灰度覆盖率并以 `(c, c, c, c)` 写入共享 RGBA 图集（等价 Vanilla `rendertype_text_intensity.fsh`），通过 `bearing_left` / `bearing_top` 保持浮点字形框。
4. **图集与度量算术**：位图保留原始整数像素与倒数缩放；`advance` 乘积按 `f32` 舍入后加 `0.5` 转 Java `int` 加 1；`Font.width` 返回 `ceil(sum)`；`space` 拒绝非有限 `f32`；超大位图单元（`> 256`）烘焙为完整缺字且保留烘焙前原始度量供 `nonFishy` 与混淆桶使用；`EmptyGlyph` 不提交位图但保留 `advance` 与装饰线。

### 2. 分行、旧版 `§`、组件阴影与装饰线局部索引

1. **源分段与模板解析**：`RichTextComponent.source_styled_parts` 保留原始节点边界（首项样式传给兄弟节点，`extra` 与翻译参数各自继承所属节点）；模板分解校验 `%s` / `%1$s` / `%%`，非法格式退回原文；语言查表按键存在性决定回退。
2. **统一分行与旧版 `§` 接线**：`hud::text_layout` 按 `StringSplitter.splitLines` 仅以 ASCII 空格与换行符切分（宽度至少为 1，首个非零 `advance` 允许超宽，保留尾换行空行）；书本编辑、阅读、讲台、进度、告示牌、对话框、提示框、HUD 覆盖、世界文本与状态效果面板均在同一 run 构造点调用 `visit_legacy_text` / `visit_legacy_utf16` 完成 `§` 解析，随后共用同一组 run 驱动宽度、断点、命中热区与绘制；所有原始输入框值走 `hud_raw_string_runs` 保持不解析 `§`。
3. **组件阴影与绘制顺序**：`ComponentStyle.shadow_color` 保留完整 ARGB（显式零值覆盖父样式，文字 alpha 非 255 时按比例缩放阴影 alpha；全零无阴影，alpha 为 0 且 RGB 非 0 仍保留几何与深度偏移）。逐字形依次提交「阴影 → 粗体阴影 → 主字形 → 粗体主字形」，整行末尾再提交全部装饰线。
4. **装饰线子序列局部索引**：下划线/删除线在 `position == 0` 处左扩 1 px；`bbb_render_types::text_visual_order::visual_order_indexed` 计算 `(bidi run) ∩ (Style 段)` 内的 UTF-16 局部下标。`HudSubsequenceKey`（HUD/屏幕）与 `WorldTextEventIds`（地图/告示牌/TextDisplay/实体名）将交互事件身份纳入分段边界，确保仅事件变化时也重启局部索引。

### 3. 二维字形点击区域与聊天页布局

- **二维字形命中（`hud_component_text_hit_geometry`）**：与绘制共用同一次 pen 遍历，先测 `prepared bounds`（仅并集可见字形与下划线/删除线，纯空白行无 bounds），再逆变换 pose 对 `HudGlyphArea`（含 `EmptyArea` 滞后访问）做左/上闭、右/下开判定；按 `includeInsertions` 分为普通点击与 Shift 插入两路扫描。聊天与书本按各自可见行集合裁定（不设额外 scissor），命中执行校验代际与修订号（见[聊天编辑、命令语法与 UTF-16 文本](client-chat-and-command-text.md#4-chatscreen-文本与生命周期)）。
- **聊天页布局与滚动同步（`HudChatGeometry` / `HudChatScroll`）**：消费启动选项 `chatScale`、`chatWidth`、`chatHeightFocused`、`chatHeightUnfocused`（默认 `0.4375`）、`chatLineSpacing`、`chatOpacity` 与 `textBackgroundOpacity`，按 Vanilla 整数公式计算页宽（`floor(pct * 280 + 40)`）、行高（`(int)(9 * (spacing + 1))`）、每页行数与 `scale(s, s).translate(4, 0)` 变换；聚焦时绘制双列滚动条（`newMessageSinceScroll` 为真取 `0xCC3333`，否则 `0x3333AA`）、`modified` 图标、截断省略号限制提示与队列提示。渲染器每帧按变化回报 `HudChatScrollPage` 与 `HudChatAppendedLines`，驱动 `scrollChat` 100 行裁剪与关屏清零。

### 4. 地图与世界文本呈现

- **地图标记名称**：物品框、第一人称与制图台共享同一字形集；按逻辑宽计算缩放 `clamp(25 / width, 0, 6 / 9)` 后再做视觉重排。背景以 `[-1, -1, 0, 9]` 起步按每个视觉字形更新左右界并提交于 `z = -0.01`（alpha `128/255`）；粗体偏移 `z = 0.001`、主字形有阴影偏移 `z = 0.03`、装饰线 `0.01 / 0.04`。物品框与制图台仅显示 `renderOnFrame` 装饰，第一人称保留全部合法装饰。
- **告示牌、TextDisplay 与名字牌**：告示牌保留首行截断与各 `WorldTextRun.font`；TextDisplay 与名字牌复用已布局不可变网格，相机移动仅对 `see-through` 层按距重排索引，资源代际变更时同步失效缓存。

## 生命周期和 fallback

- **原子资源代际**：启动与资源重载先构建 `HudFontAtlasImage` 与度量表交给 `NativeItemRuntime` 和 GPU 上传（不长期驻留 CPU RGBA）；失败时启动拒绝、重载完整回退旧代际。物品数量复用当前 default 字体数字度量与图集。
- **逐文件与提供者错误隔离**：单文件 JSON 编解码失败警告跳过；PNG 使用共享 `bbb_client_assets::image` 校验 chunk CRC（CRC/SHA-1/I/O/预算失败硬拒绝，格式内容错误标记 `ResourcePngContentInvalid` 供字体隔离）；`unihex` 坏 `.hex` 内容隔离读完后续条目，CRC/展开预算硬拒绝；`ttf` 缺文件或非法 SFNT/cmap 返回 `TtfProviderInvalid` 隔离，I/O/完整性/`16 MiB` 超限硬拒绝。自定义字体失效回退缺字字体，`default` 字体失效或完全缺失拒绝加载。

## BBB 实现映射

| 语义所有者 | 生产路径 | 边界职责 |
| --- | --- | --- |
| 字体提供者与共享图集 | `crates/bbb-client-render/src/item_model/font` | `providers`、`unihex`、`ttf`（`ab_glyph`）、`missing` 与图集装箱 |
| 字形度量与双向重排原语 | `crates/bbb-render-types/src` | `hud_glyphs`、`text_visual_order`（`visual_order_indexed`） |
| 统一字形遍历与 HUD 排版 | `crates/bbb-client-render/src/text_mesh.rs`、`crates/bbb-client-render/src/hud` | `TextPen`、`text_layout`、`chat_geometry`、`text_draws` 与点击热区 |
| 组件文本、旧版 `§` 与事件 ID | `crates/bbb-client/src/component_text.rs`、`crates/bbb-client/src/runtime` | `visit_legacy_component_runs`、地图/告示牌/进度/HUD 文本投影与 `WorldTextEventIds` |
| 聊天页滚动与输入联动 | `crates/bbb-client/src/input` | `HudChatScroll`、滚轮/翻页同步与命中消费 |

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 位图与空白提供者 | `BitmapProvider`、`SpaceProvider` | `font::providers` + 共享图集 | `aligned` | 资源栈编解码、PNG 内容隔离、预算与 `nonFishy` 测试 |
| 引用与选项过滤器 | `FontManager` 依赖与过滤器合并 | 字体提供者依赖图 | `aligned` | 整文件隔离、外层过滤覆盖、循环引用与隐藏候选预算测试 |
| TrueType 字体提供者 | `TrueTypeGlyphProviderDefinition` / `TrueTypeGlyphProvider` | `font::ttf`（纯 Rust `ab_glyph` 惰性光栅化） | `equivalent` | `font/` 前缀、度量、oversample/shift/skip、灰度覆盖与重载测试 |
| Unihex 解析、覆写与度量 | `UnihexProvider` | `font::unihex` | `aligned` | ZIP/HEX/边界/度量、全码点目录与混淆重载测试 |
| 有符号浮点度量、分行与旧版 `§` | `Font.width`、`StringSplitter`、`StringDecomposer` | `hud_glyphs`、`component_text`、`hud::text_layout` | `aligned` | 分行、源分段、所有组件/纯字符串 `§` 接线与 raw 输入隔离测试 |
| 二维字形点击区域与聊天页布局 | `ActiveTextCollector`、`ChatComponent` | `hud::text_layout::chat_geometry`、`hud_component_text_hit_geometry` | `equivalent` | 字形热区、缩放/选项公式、滚动条、`modified` 图标与 GPU 读回测试 |
| 双向重排、阴影与装饰线局部索引 | `FormattedBidiReorder`、`PreparedTextBuilder` | `text_visual_order`、`text_mesh`、`hud::text_draws` | `equivalent` | 阿拉伯语塑形、彩色半透明阴影与 `(bidi run) ∩ (Style)` 局部索引测试 |

## 结构性不变量

- 过滤器必须在提供者可见性判定前应用；`reference` 展开不得打乱原有提供者顺序。
- 首供优先（first-provider-wins）不得受哈希表遍历顺序或图集装箱顺序影响。
- `unihex`、`ttf` 与 PNG 均须受有界安全预算约束，不得因单个提供者隔离而放过 I/O、CRC 或内存预算硬错误。
- 所有文本消费者共用同一字形度量表与图集代际，严禁为特定字符集硬编码宽度或维护平行字体缓存。

## 聚焦测试（focused tests）和真实证据

| 领域 | 代表测试与命令入口 | 验证边界 |
| --- | --- | --- |
| 提供者、Unihex 与缺字 | `cargo dev test -p bbb-client-render font::` | 整文件隔离、循环引用、过滤器合并、`unihex` 覆写与预算、缺字页回滚 |
| 纯 Rust `ab_glyph` TTF | `cargo dev test -p bbb-client-render truetype` | 5 字形自校验 TTF 夹具度量、`oversample`/`shift`/`skip`、灰度覆盖、重载释放与负 bearing（设 `BBB_TRUETYPE_FONT` 可测系统字体） |
| 旧版 `§` 与组件投影 | `cargo dev test -p bbb-client legacy_component`、`cargo dev test -p bbb-client-render social_search_box` | 告示牌/对话框/提示框/HUD/世界文本/计分板/效果面板 `§` 接线与输入框 raw 隔离 |
| 装饰线局部索引与阴影 | `cargo dev test -p bbb-render-types local_index`、`cargo dev test -p bbb-client-render shadow` | `(bidi run) ∩ (Style)` 局部下标、阿拉伯语塑形、事件身份重开索引与半透明阴影 GPU 读回 |
| 二维字形点击与聊天页 | `cargo dev test -p bbb-client-render chat_click`、`cargo dev test -p bbb-client-render chat_options`、`cargo dev test -p bbb-client chat_wheel` | 字形热区、两路点击/插入扫描、缩放与选项公式、滚动条、`modified` 图标、队列提示与 100 行同步 |

## 当前差异

BBB 在桌面与 `wasm32` 目标统一使用纯 Rust `ab_glyph` 按需光栅化 TrueType 字体，以 RGBA 灰度覆盖率复刻 Vanilla FreeType 的可观察度量与渲染效果，不加载原生 FreeType 动态库；同时对 TTF/Unihex/PNG 施加显式文件体积与派生内存上限。[客户端账本](../../../unsupported-client-features.md)当前无未关闭的字体或文本 `# Todo` 缺口。

## 账本归属

本页对应机制 `ui-font-provider-pipeline`；新确认的生产差异与产品范围裁决统一登记于[客户端账本](../../../unsupported-client-features.md)，推进原则见 [`AGENTS.md`](../../../../AGENTS.md)。

## 精确重开条件

当目标版本变更字体提供者编解码、度量公式、旧版 `§` 消费面或聊天页布局规则，或新增文本消费者绕过共享字形映射表时，须重新核验本机制。

## 相关页面和源码索引

- [聊天编辑、命令语法与 UTF-16 文本](client-chat-and-command-text.md)：`ChatScreen`、命令语法着色、补全与 `JavaString` 编码边界。
- [客户端世界生命周期](../world/client-world-lifecycle.md)：`bbb-client-world::local_time` ICU 区域与时区日期格式化。
- [离线资产可信链](../resources/client-offline-asset-trust.md)：统一资源栈与字体资产来源。
- Vanilla：`FontManager`、`FontSet`、`FontOption`、`UnihexProvider`、`TrueTypeGlyphProvider`、`TrueTypeGlyphProviderDefinition`、`GlyphBitmap`、`FontTexture`、`ChatComponent`、`ActiveTextCollector`
