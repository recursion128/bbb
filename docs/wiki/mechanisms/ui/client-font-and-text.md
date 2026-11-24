# 客户端字体与文本：字体提供者、字形度量与图集

机制 ID：`ui-font-provider-pipeline`

## 机制目的和边界

本机制从资源包字体 JSON 解析 `bitmap`、`space`、`reference`、`unihex`、`ttf` 字体提供者（provider）与 `filter` 过滤器，按 Vanilla 提供者优先级生成字形度量、位图与共享图集，供 HUD、屏幕、地图与世界实体文本统一使用。Unicode 双向重排（bidi）/塑形、旧版 `§` 格式码调用面与二维字形点击几何均与本页共享同一套字形度量；聊天输入编辑、命令语法着色、UTF-16 `JavaString` 编解码与 SNBT/命令 Packrat 语法见[聊天编辑、命令语法与 UTF-16 文本](client-chat-and-command-text.md)；ICU 区域与时区日期格式化属于独立机制 `ui-local-time-icu`，不在本页边界内。

## Vanilla 26.1 权威实现

`FontManager` 收集字体构建器并解析 `reference` 引用依赖，将外层与被引用提供者的 `filter` 合并，外层同名选项覆盖内层。单份字体 JSON 的完整编解码失败会丢弃该文件；引用解析先于过滤，任一缺失引用会使所属字体整体失效。`FontSet.selectProviders` 先按 `FontOption.Filter` 筛选，再由 `computeGlyphInfo` 按提供者顺序取首个字形，并另选用于聊天输入框的 `nonFishy` 字形。

`UnihexProvider` 遍历 ZIP 中的 `.hex` 条目，解析 4/5/6 位码点与 32/64/96/128 位十六进制位图数据（对应 8/16/24/32 像素宽），相同码点后读覆盖先读。定义中的 `size_overrides` 按声明顺序先从自动扫描集合中移除并固定左右边界，其余字形扫描非零像素列。字形固定高 16、过采样率（`oversample`）为 2，水平步进（`advance`）为「宽度 / 2 + 1」，粗体与阴影偏移为 `0.5`。`GlyphBitmap` 由 `FontSet` 拼接进共享图集，边缘保留采样间距。

`TrueTypeGlyphProviderDefinition` 的 `file` 是字体资源标识，加载时添加 `font/` 前缀（`TrueTypeGlyphProviderDefinition.load`）；编解码字段包括 `size`（默认 11）、`oversample`（默认 1）、`shift`（默认 `[0, 0]`，每维范围 `[-512, 512]`）与 `skip`（字符串或字符串列表，列表先拼接再取 `codePoints()`）（`TrueTypeGlyphProviderDefinition.CODEC` / `SKIP_LIST_CODEC`）。Vanilla 在 `TrueTypeGlyphProvider` 构造器中调用 FreeType：以 `Math.round(size * oversample)` 设定像素字号（`FT_Set_Pixel_Sizes`），由 `FT_DivFix(pixelsPerEm * 64, unitsPerEm)` 计算定点数缩放并经 `FT_MulFix` 将轮廓量化到 26.6 定点像素坐标；再用 `FT_Set_Transform` 施加 `shiftX * oversample` 与 `-shiftY * oversample` 的 26.6 平移（仅平移轮廓、bearing 与包围盒，不改变 `advance`）；随后通过 `FT_Get_First_Char`/`FT_Get_Next_Char` 枚举字符映射表并剔除 `skip` 码点。若 `FT_Select_Charmap(FT_ENCODING_UNICODE)` 失败或 `FT_Get_Font_Format` 非 `TrueType`，则该提供者加载失败。度量取自 `FT_Load_Glyph(..., FT_LOAD_BITMAP_METRICS_ONLY | FT_LOAD_NO_BITMAP)`：`advance` 取 `glyph.advance()`，左右 bearing 与位图尺寸取控制点外扩整像素包围盒的 `bitmap_left`/`bitmap_top`/`bitmap.width`/`bitmap.rows`，再除以 `oversample` 得到 `GlyphInfo.simple(advance / oversample)`、`left / oversample` 与 `top / oversample`；零面积位图交给 `EmptyGlyph`，不在字符映射表内的码点返回 `null`。光栅化由 `NativeImage.copyFromFont` 的 `FT_Load_Glyph(..., FT_LOAD_RENDER)` 生成 8 位灰度图，因此 `GlyphBitmap.getLeft()/getRight()/getTop()/getBottom()` 为浮点 bearing，而非位图提供者的 `0`/整数 `ascent`。

### 组件文本的旧版 `§` 格式码调用面

`StringDecomposer.iterateFormatted` 是唯一把 `§` 作为格式码消费的入口。三参 `String` 重载将 `style` 同时作为当前样式与复位样式（`resetStyle`）；循环中 `§` 与后继单元成对消费，未知格式码仅消费不改写样式，末尾孤立 `§` 直接终止（`StringDecomposer.iterateFormatted`）；格式码查表经 `Character.toLowerCase` 执行，未命中返回 `null`（`ChatFormatting.getByCode`）。样式改写遵循 `Style.applyLegacyFormat`：颜色码覆盖颜色并清除五种装饰线/粗斜混淆标志、保留字体/点击/悬停/插入/阴影，`§r` 恢复传入的 `resetStyle`。

客户端中组件文本进入该路径的位置是视觉顺序（visual order）边界，而非 `FormattedText.visit` 本身：`MutableComponent.getVisualOrderText` → `ClientLanguage.getVisualOrder` → `FormattedBidiReorder.reorder` → `SubStringSource.create`，后者通过三参 `iterateFormatted` 同步生成纯文本与逐 UTF-16 单元的字符样式，复位样式为该片段继承后的样式。因此「走 `FormattedCharSequence`」并不意味着跳过 `§` 解析：序列产出时已完成解析，下游只按纯文本索引消费。

分行分为两段：`StringSplitter.splitLines` 的测量阶段以 `initialStyle` 作为复位样式，而切出的片段保留原始格式码、仅由 `StringSplitter.splitAt` 将样式赋给后继片段，随后视觉顺序阶段再按片段自身样式解析一次。

确认进入旧版 `§` 重载的消费者如下：

| Vanilla 调用面 | Vanilla 方法 | 复位样式与处理形态 |
| --- | --- | --- |
| 组件 → 视觉顺序（一切 `getVisualOrderText` 消费者） | `SubStringSource.create` | 该片段继承样式 |
| `StringSplitter.stringWidth(FormattedText)`（`Font.width(FormattedText)`） | `StringSplitter.stringWidth` | 经 `StringDecomposer.iterateFormatted`，复位样式为片段样式 |
| `StringSplitter.headByWidth`（`Font.substrByWidth`、省略号截断） | `StringSplitter.headByWidth` | 对片段内容直接迭代 |
| `StringSplitter.splitLines(FormattedText, …)`（`Font.split`） | `StringSplitter.splitLines` | 测量复位样式为 `initialStyle` |
| `StringSplitter.String` 入口（`stringWidth`/`plainIndexAtWidth`/`splitLines`） | `StringSplitter.plainIndexAtWidth` / `splitLines` | 纯字符串入口 |
| `Font.prepareText(String)`（`drawInBatch(String)`） | `Font.prepareText` | 复位样式为 `Style.EMPTY` |
| 聊天 HUD 行 | `GuiMessage.splitLines`、`ChatComponent.addMessageToDisplayQueue`、`ComponentRenderUtils.wrapComponents` | 先按 `chatColors` 决定是否剥离，再 `splitLines(..., Style.EMPTY)`，最后转视觉顺序 |
| 书本阅读页 | `BookViewScreen.visitText` | `font.split(pageText, 114)` |
| 告示牌四行 | `SignText.getRenderMessages`、`AbstractSignRenderer.submitSignText` | `font.split(input, maxTextLineWidth).get(0)`，宽度与绘制只读解析序列 |
| 物品名、Lore 与组件提示框 | `GuiGraphicsExtractor.setTooltipForNextFrame`、`ClientTooltipComponent.create`、`ClientTextTooltip.getWidth` | `Component::getVisualOrderText` → `create(...)`，不额外分行 |
| 对话框正文 | `DialogBodyHandlers.PlainMessageHandler.createControls`、`FocusableTextWidget.updateWidth`、`MultiLineLabel.getSplitMessage` | `font.width`/`splitIgnoringLanguage` + `getVisualOrder`，点击按绘制序列派发 |
| 对话框控件与标签 | `StringWidget.visitLines`、`AbstractStringWidget.extractWidgetRenderState` | `font.width(Component)` + `getVisualOrderText()`，点击经 `ClickableStyleFinder` 走同一序 |
| 动作栏、主标题与副标题 | `Gui.extractOverlayMessage`、`Gui.extractTitle` | `font.width(Component)` + `textWithBackdrop(font, Component, …)` |
| 实体名字牌（含名下分数） | `EntityRenderer.submitNameDisplay` | `submitNameDisplay` 提交名字牌组件 |
| 文本展示实体（`TextDisplay`） | `DisplayRenderer.splitLines` | `font.split(Component, width)` 测量并分行 |
| 地图装饰名称 | `MapRenderer.render` | `font.width(Component)` 定缩放，再提交 `getVisualOrderText()` |
| 计分板侧边栏 | `Gui.displayScoreboardSidebar` | 标题与每条目的名称、分数均为组件 |
| 玩家列表页眉、页脚、名称与分数 | `PlayerTabOverlay.extractRenderState` | 全部按组件绘制 |
| Boss 栏名称 | `BossHealthOverlay.extractRenderState` | `font.width(Component)` 居中后绘制组件 |
| 容器标题与进度提示标题 | `AbstractContainerScreen.extractLabels`、`AdvancementToast.extractRenderState` | 按组件绘制与分行 |

不解释 `§` 的形态是已经解析完成的序列与原始投影：`Font.prepareText(FormattedCharSequence)`、
`StringSplitter.stringWidth(FormattedCharSequence)`、`FormattedText.visit` 与 `Component.getString`，
以及关闭 chatColors 时显式删除的 `ChatFormatting.stripFormatting`（`ComponentRenderUtils.stripColor`）。raw String 的
`GuiGraphicsExtractor.text(Font, String, …)` 先把 String 变成
`FormattedText.of` 再走 visual order，仍属 legacy 面；已解析的 `GuiGraphicsExtractor.text(Font, FormattedCharSequence, …)`
不再二次解析。
`EditBox` 的文字走 `FormattedCharSequence.forward`（`EditBox.applyFormat`），
其显示裁剪用非 formatted 的 `plainSubstrByWidth`/`plainIndexAtWidth`（`StringSplitter.plainIndexAtWidth`），
这两处必须保持不解释 code。

BBB 在这些消费者已经构造 run 的位置接线，并复用 `bbb-render-types` 的同一 scanner。
`bbb-client::component_text::visit_legacy_component_runs` 按 part 迭代并做 `SubStringSource` 侧解析
（reset = 该 part 继承样式），`legacy_resolved_runs` 是同一 pass 的 run 列表形态，
`hud_text_style` 负责 decoration 投影；chat/advancement/book 继续使用 renderer 的 `resolve_parts`
与原始 UTF-16 入口。已接通：

- 告示牌：`sign_scene::bake_sign_text_face` 经 `component_text::world_text_runs` 先消费 code，`truncate_sign_line_runs_to_width`
  的首行断点、`sign_line_runs_width` 的居中宽度与 baked quads 因此读到同一组 run
  （`AbstractSignRenderer.submitSignText`）。
- dialog 正文与 widget/label：`dialog_screen::widgets` 的 `wrapped_text_lines` 与
  `dialog_text_runs` 共用同一 pass，分行、drawn runs、glyph-area click span、label 宽度与
  居中 x 都读同一组字形（`MultiLineLabel.getSplitMessage`、`FocusableTextWidget.updateWidth`、
  `StringWidget.visitLines`）；`§r` 回到该 part 的继承样式，颜色清除五种 decoration
  但保留 font/click/shadow。
- tooltip：renderer 在 tooltip 行步骤解析（`hud_resolved_tooltip_runs`），物品名/lore 与组件悬浮文本
  的宽度和 glyph pass 都读解析后的行；已分行的行由 `wrap_parts` 自行消费
  （`GuiGraphicsExtractor.setTooltipForNextFrame`、`ClientTextTooltip.getWidth`）。
- HUD 覆盖文本：`render_extract::hud_component_text_runs` 从这一处解析，覆盖 action bar、title/subtitle、
  玩家列表 display name 与命令确认正文；renderer 的居中宽度与 glyph quads 读同一列表
  （`Gui.extractOverlayMessage`、`Gui.extractTitle`）。
- 世界名与 display 文本：`localized_world_component_runs`（地图装饰名）、`display_text_runs`
  （TextDisplay）、`entity_name_tags::entity_name_tag_text_runs`（实体名与 below-name）
  都在各自的分行/缩放之前解析，所以 `item_frame_map_text_width` 的 scale、TextDisplay 的
  `split_display_text_lines` 结果与 baked quads 与字形一致
  （`MapRenderer.render`、`DisplayRenderer.splitLines`、`EntityRenderer.submitNameDisplay`）。
- 计分板与玩家列表：`hud_scoreboard_text_runs` 与 `hud_component_text_runs` 解析后，
  sidebar 的右缘 `left`/`title_x` 与逐行 glyph pass 读同一宽度
  （`Gui.displayScoreboardSidebar`、`PlayerTabOverlay.extractRenderState`）。
- boss bar 名与容器/toast 标题：`hud_boss_bar_from_world_state` 与
  `hud_styled_runs_from_component_runs` 解析（`BossHealthOverlay.extractRenderState`、
  `AbstractContainerScreen.extractLabels`、`AdvancementToast.extractRenderState`）。

focused 证据：`legacy_component_runs_consume_codes_with_the_part_style_as_reset`（run 级：颜色清除
decoration、`§r` 回到继承样式、未知 code 与尾 `§` 消费）、
`legacy_resolved_runs_split_at_each_code_and_merge_only_equal_styles`、
`legacy_component_runs_never_pair_codes_across_parts`、
`sign_line_consumes_legacy_codes_for_runs_break_and_baked_glyphs`（首行断点、居中宽度与 baked quad 数）、
`dialog_body_consumes_codes_before_lines_runs_and_click_spans`（分行、drawn runs、click span 与命中 x）、
`dialog_body_reset_after_a_colour_returns_to_the_part_style`、
`dialog_widget_labels_consume_codes_before_width_runs_and_centering`（label 宽度、居中 x 与 runs）、
`tooltip_line_legacy_codes_style_runs_width_and_drawn_glyphs`（resolved run、tooltip 宽度与 glyph 绘制顺序）、
`action_bar_and_title_runs_consume_legacy_codes_with_the_root_style_as_reset` 与 renderer 侧
`action_bar_and_title_draw_the_resolved_runs_that_centred_them`（居中 origin 与 glyph advance 序列）、
`text_display_consumes_legacy_codes_before_the_line_split_and_glyphs`（分行数与 content_width）、
`entity_name_tag_consumes_legacy_codes_before_width_and_baked_glyphs`（submission width 与 baked quad 数）、
`localized_world_component_runs_consume_codes_before_the_map_text_width`（地图文字 scale 宽度）、
`scoreboard_sidebar_runs_consume_legacy_codes_with_the_inherited_style_as_reset` 与 renderer 侧
`scoreboard_layout_reads_the_resolved_runs_the_glyph_pass_draws`（右缘 `left`/`title_x`）、
`player_list_runs_consume_legacy_codes_for_header_and_display_name`、
`boss_bar_name_runs_consume_legacy_codes_with_the_root_style_as_reset`、
`container_label_and_toast_component_runs_consume_legacy_codes`。

纯字符串绘制面已按 Vanilla 的两支拆开：`hud_coded_string_runs` +
`hud_coded_string_width` / `push_hud_coded_string_text` 服务
`text(Font, String, …)`（消费 code）形态，`hud_raw_string_runs` /
`hud_raw_string_width` / `push_hud_raw_string_text` 只服务 `EditBox` 值本身的字形 pass
（`FormattedCharSequence.forward`，`EditBox.applyFormat`）与
`plainSubstrByWidth`/`plainIndexAtWidth`（`StringSplitter.plainIndexAtWidth`）的裁剪索引。
命令补全列表两侧都消费 code
（`CommandSuggestions.showSuggestions`、`CommandSuggestions.SuggestionsList.render`），
告示牌编辑器的行、标题、preedit、光标与选区前缀同属消费一侧
（`AbstractSignEditScreen` 构造器 / `extractSignText`、`IMEPreeditOverlay` 构造器）。
共享 helper 不再自建 plain run：`push_hud_debug_options_text` 一族、
`push_hud_pause_button` 与三处 `push_button` 都接受 run 列表，本地构造的 code-free
调用点显式传 `hud_raw_string_runs`；同一个调用点服务两种输入时按 Vanilla 分流，例如
社交搜索框（值是 raw、提示是 `EditBox.extractWidgetRenderState` 的 Component）与暂停屏
`custom_dialog_label`（Component 投影，缺省取 `"Report Bugs"` 字面量）。移动效果面板将名称与时长各解析一次后同时驱动面板宽度（`EffectsInInventory.extractBackground`）、经 `hud_clip_styled_text_runs` 执行 `ComponentRenderUtils.clipText` 语义的截断以及字形绘制（`EffectsInInventory.extractText`）。聊天与命令输入行、书页编辑行的原始编辑值、社交搜索框的输入值与容器文本输入框均保持不解析 `§` 格式码。
F3 overlay、debug chart 与游戏模式切换文本已入消费一侧，宽度与绘制读同一趟
`iterateFormatted`（`Font.width(String)`）；游戏模式切换的两行按
`GuiGraphicsExtractor.centeredText(Font, Component, ...)` 读同一份
run 列表，其中提示行由生产者按 `GameModeSwitcherScreen.extractRenderState` 建成
`debug.gamemodes.select_next` 加一个 keybind 参数（`key.debug.switchGameMode`，
`Options.java:660`）：参数经 `KeybindResolver.keyResolver`（`KeybindContents.getNestedComponent`，
客户端在 `Minecraft` 构造器中装的正是 `KeyMapping::createNameSupplier`，也就是
`getTranslatedKeyMessage` 背后那条）解析成当前绑定键的显示名，AQUA 只落在这一段，
句子那段不带颜色；
社交玩家状态走 `hud_coded_string_runs_with_style`，因为
`StringDecomposer.iterateFormatted(text, style)` 把调用方样式同时当作当前样式与 `§r`
的复位样式，对应 `PlayerEntry.java:61-65` 的
`Component.translatable(..).withStyle(ITALIC)`。
renderer 侧证据：`stats_row_name_and_value_draw_the_resolved_runs_that_measured_them`
（coded 行名/行值的着色字形列与右对齐 x）、
`social_search_box_value_keeps_its_section_glyph_and_hint_consumes_codes`
（同一调用点的 raw 值与 coded 提示）、
`mob_effect_panel_width_break_point_and_glyphs_share_the_resolved_name`
（面板宽度、断点与保留下来的 run 列表）。

## 决策输入与状态所有权

统一 `PackResourceStack` 决定源码、索引、已选与服务器资源包的字体 JSON/ZIP/PNG/TTF 字节及优先级顺序；字体选项（`FontOptions`）决定过滤结果；提供者依赖图决定候选顺序；字形提供者持有度量与源数据，资源代际持有共享图集，文本排版持有当次码点与样式序列。

## 判断与处理顺序

1. 同名字体读取完整资源栈，高层包优先，同文件内按声明顺序排列，低层独有提供者保留。资源包过滤器在读取前生效；单份文件的全部提供者与过滤器解析成功后才贡献定义，坏字段、未知类型或非法过滤器丢弃整份文件，合法低层资源继续参与。
2. 先解析 `reference` 引用，再合并过滤器，外层同名选项覆盖内层，重复引用保留各自过滤上下文。缺失或循环引用标记整套字体失败，但继续检查兄弟候选。所有 `bitmap`/`unihex`/`ttf` 候选均先完成加载检查与预算计费，再按当前选项安装，被过滤器隐藏的提供者同样支付检查与资源预算。
3. 仅保留至少首供一个码点的活跃提供者，再分别选择普通与 `nonFishy` 字形。普通选择遵循首供优先；缺字使用独立缺字字形，不占用真实 `?` 或 `U+FFFD`；未知字体 ID 使用缺字字体，不借用默认字体的正常字符。
4. `unihex` 先按声明顺序处理 `size_overrides`，再对其余字形自动扫描非零像素列边界。
5. `ttf` 在 `crates/bbb-client-render/src/item_model/font/ttf.rs` 中统一使用纯 Rust `ab_glyph` 解析轮廓，零 FreeType 调用：先校验 SFNT 魔数（`[0, 1, 0, 0]` 或 `b"true"`）、`glyf` 表与 Unicode `cmap` 子表；将有限 `oversample` 钳制至不小于 `MIN_TTF_OVERSAMPLE`（`0.1`，非有限值回退 `1.0`），以 `pixels = java_round_float(definition.size * oversample).max(1) as f32` 计算 `PxScale::from(pixels * (font.height_unscaled() / units_per_em))`，按 1/64 像素粒度取整 `shift_x`/`shift_y`；遍历字符映射表（剔除 `skip` 码点，上限 `MAX_TTF_GLYPHS = 1_114_112`），通过 `scaled.h_advance(glyph_id)` 与 `font.outline_glyph(...)` 提取每个字形的 `advance`（除以 `oversample` 后钳制到 `[-4096.0, 4096.0]`，非有限值归 `0.0`）、`bearing_left`/`bearing_top`（`left / oversample`、`top / oversample`）及非零面积时的栅格尺寸 `TtfGlyphBounds`（宽高上限 `TTF_PAGE_MAX_DIMENSION = 8_192`），零面积或无轮廓字形记录 `bounds: None`（对应 `EmptyGlyph`）。
6. 按字形的 `top`/`ascent`/`advance`/`oversample` 拼接共享图集。位图提供者保存整数原始像素尺寸，四边形按 `pixelSize / (1.0 / pixelScale)` 保留浮点与符号而不预先取整；`advance` 乘积先按 `f32` 舍入，再用 `f64` 加 `0.5` 转 Java `int` 并回绕加 1；`top` 先把 `ascent` 转浮点再从 `7.0` 扣除。整行 `Font.width` 返回 `ceil(sum)` 的有符号整数；普通截断游标按浮点剩余预算逐字扣减，首次超宽即停止，负宽不在度量入口钳零。`space` 定义仅接受有限 `f32`，超界值明确拒绝。

HUD chat、plain 和 multiline 的分行由 `hud::text_layout` 的范围切分器拥有。
按 `StringSplitter.splitLines` 与 `LineBreakFinder`，仅 ASCII 空格与换行符充当分隔符，
每次只消费一个分隔符，不裁掉其余前后空白；宽度至少为 1，累加实际 `f32` advance，
首个非零 advance 字符允许超过行宽。分行保留原始字符范围，组件样式、font 与交互目标
随范围投影；显式尾换行产生空行。空组件不产生行，plain 与编辑输入适配器保留空文本一行。
`text_layout::wrapping_tests` 覆盖零宽前缀、小数/负宽、ASCII 与其他空白、跨样式目标和
multiline 字符索引。`hud_plain_text_line_ranges` 还为原生 book/dialog、书页绘制和
code-of-conduct 提供同一分行范围；book 与 dialog 使用普通 glyph map，聊天输入继续独立
使用 nonFishy map。书页保留空输入、尾换行和软换行边界的光标归属，依据
`MultilineTextField.reflowDisplayLines`。advancement 的原生布局直接接收当前 prepared font book，
标题、进度文字与描述使用真实字体宽度；描述按 `AdvancementWidget.findOptimalLines` 依次测试
0、10、-10、25、-25 的宽度偏移，以逻辑 `f32` 最大行宽选候选，完成行再按视觉顺序度量。
`runtime::advancement_text` 拥有悬浮文本布局，`HudInventoryTextLabel.component_runs`
将混合字体保留至统一 component geometry。分好行的组件标签不以最终宽度二次裁切，避免
正 pen 前缀与负 pen 尾部导致合法字形消失；旧标签的 width 和输入框合同保持原有含义。

告示牌依据 `AbstractSignRenderer.submitSignText` 只保留第一次分行的内容；newline 立即截断，
零宽前缀不排除首个超宽 glyph。TextDisplay 已保留相同首非零/ASCII/f32 边界并在分行后
执行 bidi。告示牌投影保留每个 `WorldTextRun.font`，分行、视觉居中与 glyph UV 使用
同一 prepared font book；每帧混淆字形只从该 slot 所属 font 的池选择。所有字体复用
资源代的共享 atlas；未知字体使用 book 的 missing 字形，不回退默认字体的正常字符。
字体 reload 的已提交资源更新会失效稳定方块实体缓存，随后重新投影告示牌文本。
告示牌 GPU 几何在文字、可见集合或实际 outline 结果变化时更新；可见混淆文字继续重建动画。
TextDisplay 复用已有字体布局的不可变几何，绘制输入变化时才变换并上传世界顶点；相机平移
只排序并上传需要排序的索引，see-through 字形保持提交顺序。资源代际拥有这些驻留 buffer。
名字牌沿同一布局保留世界网格；普通层保持提交顺序，see-through 行只随距离重排索引，
等距的分数和名字保持相对顺序。字体、光照、样式、姿态与可见成员变化才更新相应几何。
`sign_face_projection_retains_component_font_for_wrap_filter_and_rebake` 覆盖自定义字体、
filtered text 选择及重新烘焙；item-render 定向测试覆盖字体度量、UV 与混淆池选择。
协议的 `RichTextComponent.source_styled_parts` 保留原始节点边界；advancement 的实际显示
使用带语言资源的本地化入口。两者都将根组件列表的首项样式传给后续 sibling，
而 `extra` 成员与翻译参数各自继承所属节点，避免成员覆盖泄漏。既有 `styled`、
纯文本和序列化投影保持兼容。源分段查询不执行格式码解析，也不能
代替翻译模板的分段解析。本地化入口使用共享模板分解器，保留字面文本、转义百分号、
参数组件与 sibling 的源分段；非法格式或越界参数使整条模板退回原文，显式参数索引
不推进隐式索引。商人标题、附魔提示、死亡分数与聊天模板使用同一校验入口。
下游分行与绘制仍须传递这些边界。源分段同时是投影的 run ordinal：
`TranslatableContents.decomposeTemplate`/`getArgument`/`visit` 把字面片段与每个 `with` 参数分别
visit 并给同一个 `currentStyle`，所以样式完全相同的相邻分段仍是两个 run；按 `Style`
相等合并只发生在 bidi 之后为字体切 piece 的那一步（`SubStringSource.substring`、
`FormattedBidiReorder.reorder`），投影不合并因此不是差异。
语言查询用键是否存在决定 fallback，显式
`key → key` 与空译文都保留；组件、GUI、聊天、死亡分数和游戏规则使用该查询。
共享 `visit_legacy_text` 适配 Rust 字符串，`visit_legacy_utf16` 保留分段中的孤立代理单元；
二者使用同一解析实现，按 UTF-16 单元消费格式码，输出原 UTF-16 与 scalar 索引；
调用方显式提供当前样式、reset 样式及格式应用规则。终末之诗已替换独立解析器，
覆盖补充字符被格式码消费后的替换字符和颜色清除装饰。Advancement 悬浮标题与描述
先按当前语言解析模板与参数，再接入原始 UTF-16 分段分行：分行使用根 reset，保留原文直到切行，剩余片段继承
切分样式；每行随后用逐片段 reset 解析，再走既有 bidi、字体度量与绘制。
选中页签标题、窗口标题、空页面提示和完成按钮均按当前语言解析，使用不分行的
共享格式入口生成文字与度量；进度与最大进度宽度共用 `advancements.progress` 模板。
聊天装饰参数优先读取保存的原始组件，在当前语言与装饰样式下解析；模板字面片段
保持独立，逐片段过滤控制字符，不与带 insertion 的参数合并。聊天显示与旁白复用
同一参数投影，点击目标按完整投影的 `usize` ordinal 反查，覆盖超过 65,535 个片段的目标，
聊天与书本源 run 均不再因格式片段增加而在 `u16` 转换处静默丢失。聊天消息已接入原始分段分行器：目标编号和视觉样式一同参与 reset，解析后才做 bidi
与命中区域计算；分行根 reset 可以清除后续目标，未分行的逐片段 reset 恢复该片段目标。
书本阅读与讲台正文的绘制和点击查询共用此路径，保留组件片段及目标；
对应 `BookViewScreen.visitText` 的正文分行流程。页面默认黑色且显式禁用阴影，
组件自带阴影覆盖默认值，子片段继续按组件继承规则解析（`Style.withoutShadow`）。书本编辑页的分段绘制及逻辑宽度已接入格式解析，光标两侧分别解析；
书本编辑的行数限制已使用保留原始 UTF-16 范围、跨行样式与尾换行空行的格式分行；

（关于 `JavaString` 原始 UTF-16 单元保留、网络 UTF-8 编码、SNBT/命令参数 packrat 语法与补全，见[聊天编辑、命令语法与 UTF-16 文本](client-chat-and-command-text.md)。）

## 生命周期和 fallback

启动与资源重载先准备一份 `HudFontAtlasImage`，把同一 book 的 metrics 传给
`NativeItemRuntime`，再将该 atlas 交给 HUD 和地图纹理上传；物品 runtime 不重新烘焙字体，
也不长期保留 atlas 的 CPU RGBA。字体准备错误直接传播：启动失败，重载保留原 generation。
重载的新 metrics 和 GPU 字体在同一事务成功后才替换当前状态。

font catalog 随 resource generation 原子替换。单文件 JSON/codec 错误警告并跳过该层，
包括被 filter 隐藏的无效 provider；I/O、完整性、集合和 unihex 宽度预算失败仍拒绝加载。
`FontManager.loadResourceStack` 对应逐文件隔离。reference 失败、缺失 bitmap/unihex 资源、
明确的 PNG/ZIP/HEX 内容错误以 typed failure 使整 font 失效，并沿引用传播
（`FontManager.UnresolvedBuilderBundle.resolve`）。失效 custom font 从 book 排除，访问时使用缺字字体；
default 失效拒绝加载。失败不会提前跳过兄弟候选，也不会将失败缓存成空的成功字体。

unihex 在遇到 HEX 内容错误后仍有界读完该 `.hex` entry，并检查后续 entry。
CRC、I/O、声明/实际展开预算和图像预算均为硬错误；无法打开 raw entry 完成声明大小检查也硬拒绝。
非 `.hex` payload 不读取，不构成全 ZIP CRC 证明。

PNG 的共享 `bbb_client_assets::image` 从同一份有界字节快照完成 chunk CRC 检查和解码。
IHDR、IDAT、ancillary 与 IEND 的 CRC 不因 decoder 提前遇到内容错误而跳过；CRC、资源
SHA-1/大小校验、ZIP、底层读取和所有预算错误仍硬失败。完整快照中的短头、截断、零尺寸与
其他格式错误由 `ResourcePngContentInvalid` 标记，只有该类型进入 font 的失败缓存。
有效 IHDR 的尺寸先支付图像预算，后续内容失败不退款；相同 provider/reference 复用缓存。
读取期间文件大小变化或实际字节超限也不能变成可隔离错误。此处的完整性检查指 PNG chunk
CRC，不宣称校验 zlib Adler-32。

Vanilla 的 `NativeImage.read` 将头部或像素解码失败交给
`FontManager.safeLoad` 的 provider 隔离；BBB 在该隔离边界外保留资源完整性与预算约束。
custom font 失败后使用缺字字体，default font 失败仍拒绝准备；继续检查兄弟候选，避免较早的
内容错误掩盖较后的硬错误。新 generation 成功后才能恢复已修复的 custom font，旧 book 不变。

bitmap 通过共享 pack 图像解码器读取完整 PNG，不消费 sprite `.mcmeta`、帧裁剪或动画。
零/负 provider height 允许产生退化或反向 quad；任一 raw cell 维度超过 256 时，
使用完整 missing glyph 的纹理与 metrics，不继续选择后续 provider（`FontSet.wrappedStitcher`）。
UV 内缩 0.01 像素。`HudDigitGlyph.is_empty` 对应 `EmptyGlyph`，与 raw cell 尺寸独立：
space provider 使用 `empty(advance)`，bitmap/unihex/missing 字形保持可提交。
HUD、数量、地图、告示牌、TextDisplay、实体名称和 debug text 按该标记决定是否生成 bitmap，
零尺寸 cell 的退化或非有限坐标不在此层改写。空白字形不提交 bitmap，但保留 pen advance
与下划线/删除线（`EmptyGlyph.bake`）；未加载字体的查询显式返回 advance=0 的空白字形。
unihex 源页面及最终共享 atlas 在分配前计入 generation RGBA 预算；隐藏页面同样收费，
同一 provider 的成功页面和失败结果由本次加载缓存复用，页面 `Arc` 克隆不重复收费。

`ttf` 与 `bitmap`/`unihex` 共用同一资源加载与安全预算边界：定义中的 `file` 添加 `font/` 前缀后经 `PackResourceStack::get_resource` 与 `validate_resource` 解析（缺失资源或 SFNT/cmap 格式不合法返回 provider 级失败 `TtfProviderInvalid`，而 I/O、完整性校验与 16 MiB 单文件上限 `MAX_TTF_ASSET_BYTES` 仍硬拒绝）。`font::ttf`（`crates/bbb-client-render/src/item_model/font/ttf.rs`）在桌面与 `wasm32` 目标统一使用纯 Rust `ab_glyph` 解析轮廓与光栅化，不加载任何原生 FreeType 动态库（`bbb-client-assets::native_library` 的原生库验证提取仅服务 OpenAL 音频库）。

`TtfGlyphSource` 采用惰性光栅化设计，不预先烘焙整张 RGBA 页面：加载阶段保留已校验的字体原始字节 `font_bytes: Arc<[u8]>` 与字符映射度量目录，并按 `glyphs.len() * 96 + bytes.len()` 向 `reserve_derived_resource_bytes` 预留派生内存预算；参数遵循明确安全钳制——`oversample` 下限钳制为 `MIN_TTF_OVERSAMPLE = 0.1`（非有限值回退 `1.0`），像素字号 `pixels` 以 `.max(1)` 钳制，`advance` 钳制至 `[-4096.0, 4096.0]`（非有限值回退 `0.0`），单字形栅格边界宽高上限为 `TTF_PAGE_MAX_DIMENSION = 8_192`。实际需要绘制或烘焙某字形时，再调用 `TtfGlyphSource::rasterize_glyph_rgba` 经 `font.outline_glyph(...)` 按需光栅化单字形 8 位灰度覆盖率，并以 `(coverage, coverage, coverage, coverage)` 写入共享 RGBA 图集，使既有 `texture * tint` 着色器等价于 Vanilla `rendertype_text_intensity.fsh` 的 `texture(...).rrrr * vertexColor`。`HudDigitGlyph` 的 `bearing_left`/`bearing_top` 分别承载 `GlyphBitmap.getLeft()`/`getTop()`：位图/空白/unihex 保持 `0` 与整数 `ascent`，TTF 使用 `bitmap_left / oversample` 与 `bitmap_top / oversample`，因此斜体错切、粗体扩展、阴影与点击热区均作用于同一浮点字形框。

每份 book 只追加一张 5×8 缺字图，白边、透明中心，advance=6，ascent=7；
UV 使用 0.01 像素内缩（`SpecialGlyphs.java:14-20`、`FontTexture.java:43-51`）。
空 provider 列表、全坏定义文件和纯 space 字体均可建立该回退，图集尺寸预算包含缺字页面。
完全缺少 default 定义仍拒绝启动。默认 options 为 `uniform=false`、`jp=false`。

## BBB 实现映射

### 资源字符串与码点目录

font JSON 通过 `PackResourceStack::read_resource_json` 读取，保留 key/value 中由 JSON escape
产生的孤立 surrogate；合法高低对合成一个补充平面 codepoint。读取仍校验资源完整性、原始 UTF-8、
文件大小、全树深度、节点数、解码字符串字节和 generation 预算。普通资源 ID、provider type 与
option 名使用有效 UTF-8 查询；bitmap grid、space key、unihex override 端点使用完整 codepoint
迭代。未消费字段中的孤立 surrogate 不使合法 provider 文件丢失，坏字段仍按整文件隔离规则处理。
依据为 `ExtraCodecs.java:222-225`、`BitmapProvider.java:55-73` 和 `UnihexProvider.java:436-448`。

bitmap、space、unihex 与共享 glyph map 使用 `u32` key。HEX 保留 `0..=0x10FFFF`，包括
D800–DFFF；同 ZIP 内后条目覆盖前条目，有序 inclusive override 先匹配生效，跨 provider 首供优先。
surrogate glyph 同样支付目录、栅格和图集预算，不能通过丢弃它们减少计费。bitmap 的 U+0000
仍为空 slot；unihex override 端点必须严格递增。对应 `CodepointMap.put` 与
`UnihexProvider.java:436-448`。

普通文本查询继续接收 Rust scalar，目录中的 surrogate 不与 U+FFFD 或补充 codepoint 混同。
它们可以进入 ceil-advance 混淆桶并被普通文本选中；图集、UV 和 metrics 经同一字体 book 送往
HUD/world consumer，重载时候选池随 book 替换。`FontSet.selectProviders` 和 `FontSet.getRandomGlyph`
决定这一可达性。Vanilla 普通文本迭代会合并高低对并将孤立 surrogate 替换为 U+FFFD
（`StringDecomposer.feedChar`）；不能由此删除资源目录中的 glyph。

ZIP/JSON 到 atlas 的测试覆盖各 provider、重复与覆盖、计费和重载；HUD 整帧 GPU 测试证明
普通字符的混淆样式实际绘制 surrogate glyph，并在重载后移除旧候选。

### 字体准备与消费

`bbb_client_render::item_model::font::providers` 解析 provider/reference/filter；
`font::providers::codec` 拥有逐文件 typed codec；`font::unihex` 有界读取 ZIP/HEX 并生成 glyph；
font atlas runtime 合并 bitmap/unihex 和 `font::missing` 缺字页面；
`bbb_render_types::hud_glyphs` 保留 UV、advance、top、style quads；
`bbb-client`/`bbb-client-render` 的 HUD、screen、map、sign、TextDisplay 等消费同一
`HudFontGlyphMap`。物品数量从当前 default map 派生数字 metrics，并复用同一 HUD 字体纹理；
不再单独截取 ASCII PNG。数量字形的 scale、ascent、shadow offset 与普通文本共用 quad 生成，
位置遵循 `GuiGraphicsExtractor.itemCount`；重载失败时字体与派生数字一起恢复。
definition 与 unihex ZIP 不再经过字体专用 indexed fallback；它们和 language/sound 一样
只使用默认 `vanilla` pack 所在的共享 stack。

HUD 和 world text 共用 `bbb-client-render/src/text_mesh.rs` 的 `TextPen`/`PlacedGlyph`
glyph walk。`hud/text_layout.rs` 与 sign、TextDisplay、map 等消费者根据同一 metrics/style 和
视觉顺序生成各自提交值；字体 provider/atlas、行布局与 GPU 提交仍属各自阶段。

已排版的 HUD glyph/effect 共用 `hud/text_draws.rs` 的发射入口；聊天提供 pose 与量化后的
淡出 alpha。经验数字在字体坐标中排版一次，五个绘制 pass 分别应用原点和颜色，保留
glyph/effect、描边与中心以及批次的原有顺序。

### 地图标记名称

协议和 world 保留 `RichTextComponent`。`item_frames`、`item_models/first_person`、
`cartography_preview` 通过共享本地化投影生成 `WorldTextRun`，将继承后的 font、RGB、阴影 ARGB 和五种
字体样式传给 map baker；启动/reload 准备的完整字体集与上传 atlas 由三条路径共用。
逻辑 runs 按各自 font 的 bold-aware advance 累加后整体 ceil，决定居中与
`clamp(25 / width, 0, 6 / 9)`；随后携带样式做 visual order，不能先重排再测量。
依据为 `MapRenderer.render` 与 `GuiGraphicsExtractor.map`。

每个字形用 resident unit quad、局部变换、UV 和 RGB 实例提交，粗斜体保留实际角点；
绕序与地图正面一致，避免世界和第一人称的 Back cull 剔除文字。
物品框与第一人称第二个 bold quad 增加局部 z=0.001；有阴影时 main glyph 另增加 z=0.03，GUI 字形保持平面。
装饰线以 solid-color 实例排在整行字形之后；阴影深度为 0.01，main 有阴影时为 0.04，
否则为 0.01。empty glyph 仍推进 pen 并生成装饰线。
`EffectInstance` 忽略 flat 参数，所以 GUI 装饰线也保留该深度，但 GUI 管线不做深度测试
（`BakedSheetGlyph.EffectInstance.render`）。
混淆从对应 font 的 ceil-advance 桶取替代字形，绘制 pen 与 effects 使用替代后的 metrics，
不改变用于居中的逻辑宽度；现有地图缓存帧号驱动可重放的逐帧随机源。
依据为 `Font.getGlyph`、`Font.PreparedTextBuilder.accept` 与 `BakedSheetGlyph.renderChar`。

物品框和制图台只显示 `renderOnFrame` 的 decoration；第一人称保留所有合法 decoration。
地图与图标先提交，文字保持 order 1，GUI 预览仍插入既有背景层和锁定覆盖层之间。
GUI 的过滤依据是 `GuiGraphicsExtractor.map`，不能套用第一人称地图的规则。

背景独立于 run 的颜色与样式，以黑色 alpha `128/255` 的 solid 实例先于字形提交，局部 z=-0.01。
构造先记入 `[-1,-1,0,9]`，所以空名称也有背景；随后每个视觉字形（含 empty）在推进 pen 前，
以 `pen-1` 更新最左边界、以 `pen+styledAdvance` 更新最右边界。混淆后使用替代字形 metrics，
不使用逻辑宽度或可见像素 bounds 替代背景范围，也不补采样最终 pen；负宽保留这一顺序，NaN 显式传播。
world/first-person 采用普通 alpha 混合并保留深度写入与背面剔除，GUI 沿用无深度测试的 alpha 混合。
依据为 `Font.PreparedTextBuilder`、`PreparedTextBuilder.markBackground`、`PreparedTextBuilder.visit`、
`RenderPipelines.java:80-84` 与 `BlendFunction.java:10-11`。

### 组件阴影样式

`ComponentStyle.shadow_color` 保留完整 ARGB；整数读取沿用 NBT 数值窄化，四元浮点向量按
RGBA 顺序逐通道乘 255、向下取整并掩码至 8 位。子样式的显式零值也覆盖父样式，
阴影不同的相邻文本不能合并；从 typed style 编码时保留全部位并写为有符号 NBT Int。
颜色转换对应 `ExtraCodecs.java:140`、`ARGB.color`、`ARGB.colorFromFloat`
和 `ARGB.as8BitChannel`；样式规则对应 `Style.applyTo`、
`Style.equals` 和 `Style.Serializer`。原始 NBT 与派生样式仍须一致，
通用 rich component/style 反序列化不会接受不匹配的缓存。
旧 JSON 快捷栏导入仅在缓存完全匹配旧阴影投影时，从原始 NBT 重建样式；
比较包含去除阴影后重新合并的 runs、嵌套 hover、容器和 direct holder 描述，
不改原始 NBT 或其他投影。更新后仍经过完整物品校验，再写入当前 NBT 存档格式。

`HudStyledTextRun` 与 `WorldTextRun` 将该字段送入 HUD、物品提示、断线原因、地图、告示牌、
TextDisplay 和实体名称。翻译参数、悬浮组件、换行与截取保留继承后的值，合并相邻 run 时比较阴影。
`HudStyledTextRun::resolve_shadow_argb` 统一实现 `Font.PreparedTextBuilder.getShadowColor`：显式值不受
`drawShadow` 开关影响；文字 alpha 不为 255 时，阴影 alpha 与它相乘并向下取整至 8 位。
没有显式值才按开关生成 RGB 每通道整数除 4 的默认阴影。完整 ARGB 为零表示无阴影；
alpha 为零但 RGB 非零仍保留阴影几何与 main 深度偏移。

HUD 先生成每个 glyph 的 shadow、bold shadow、main、bold main，再提交全部装饰线，
不能按整行分成阴影和正文两遍；后字形的半透明阴影可以覆盖前字形。每个 glyph 只取一次混淆替代，
字形、阴影、推进与装饰线共用选中 glyph 的 metrics。世界文字保留 main 的 0.03 深度偏移和
bold 的 0.001 偏移；告示牌八向 outline 仍保留显式阴影，并按该次 pass 的文字 alpha 解析。
对应 `BakedSheetGlyph.renderChar`、`BakedSheetGlyph.renderEffect` 与 `Font.drawInBatch8xOutline`。
聊天缓存保存相同的 glyph/effect 顺序，淡出时量化 alpha；显式阴影与混淆文本即时生成几何，
避免复用旧 alpha 或随机替代。

CPU 回归覆盖零值覆盖、逐 pass alpha、0.5 offset、empty glyph effects 与世界深度；
HUD 整帧及地图 GUI/world/first-person GPU readback 检查半透明彩色阴影、覆盖顺序和透明 atlas。
相邻 part 的 click/hover/insertion 身份参与 decoration 子序列 key；HUD、告示牌、
TextDisplay 与地图装饰名在仅事件变化时也重开局部索引。上述回归不证明完整字体等价。

### 装饰线的子序列局部索引

underline/strikethrough 的左端比字形 pen 左移 1px 的条件是 `Font.PreparedTextBuilder.accept` 的
`position == 0`，而 `position` 是 `StringDecomposer.iterate` / `iterateBackwards` 交给
`FormattedCharSink.accept` 的**本子序列内** UTF-16 下标（`FormattedCharSink.accept`、
`StringDecomposer.iterate`）：forward part 的 0 在首个绘制的字形，odd run 的 0 在最后一个，
subsequence 内首个 UTF-16 unit 的下标也用于补充字符与 shaped 结果。part 由
`SubStringSource.substring` 按每个 `Bidi.getVisualRun` 切出，并在逐 UTF-16 unit 的 `Style`
变化处再切（`SubStringSource.substring`、`FormattedBidiReorder.reorder`），所以分段是
(bidi run) ∩ (Style 段)，与整行首字或纯样式分段都不同。

`bbb_render_types::text_visual_order` 的 `visual_order_indexed` 返回每个绘制字形的
该局部索引；`visual_order` 保留 LTR ASCII 恒等快路径并复用同一 ICU 管线，但 indexed 入口
不走该快路径，因为 ASCII 行也会在段落边界（`Bidi` 的每个 paragraph 各自算 run）重开索引。
`T` 相等就是 part 边界，因此 payload 若含 part 自身文本会切出 Vanilla 不切的 part；
`HudSubsequenceKey` + `HudSubsequence` 把绘制 payload 与这道 key（五种 decoration、颜色、
阴影、font、交互身份）分开，渲染端据此保留 `Style` 分段而不引入 run 文本差异。

四个世界文本与 HUD 装饰线消费者共用同一局部索引：HUD/screen 几何（`hud_resolved_text_geometry`），以及经 `text_mesh::world_text_visual_order` 与 `mesh_world_text_line` 生成的地图装饰名、告示牌、TextDisplay 与实体名；EditBox 的逻辑路径按 Vanilla 的
`FormattedCharSequence.forward` 用整行自身偏移。命中区仍与绘制字形同序同 identity。

key 里的交互身份按面分两种来源：HUD/screen 由调用方逐字形给交互索引；世界文本（告示牌、
TextDisplay、实体名、地图装饰名）没有可借的交互索引，改由 `WorldTextRun.event_id` 承担——
`bbb-client` 的 `WorldTextEventIds` 给每个互异的 click/hover/insertion 三元组发一个 id，
三元组全空不发 id、重复三元组复用同一 id，正好满足 `Style.equals` 只需区分而不需排序。
世界文本一侧的 run 合并（`component_text::push_world_text_runs`、`text_mesh::push_world_text`、
`truncate_sign_line_runs_to_width`）都把 `event_id` 纳入比较，否则仅事件不同的相邻 part
会在 key 看到边界之前先被合并掉。三面各自的 `position == 0` 证据：
`sign_run_that_only_changes_its_event_restarts_the_effect_expansion`、
`display_text_run_that_only_changes_its_event_restarts_the_local_index`、
`decoration_run_that_only_changes_its_event_expands_its_own_first_glyph`，
生产者侧 `map_decoration_name_runs_separate_the_event_triples_they_carry`。

focused 证据：`local_index_restarts_per_visual_run_like_formatted_char_sequences`、
`local_index_is_invisible_in_the_glyph_sequence_but_splits_forward_runs`（AN 与 LRE 让绘制序列
等于逻辑序但 run 仍重启索引）、`local_index_restarts_at_each_style_part`、
`local_index_counts_utf16_units_and_shaped_arabic_units`（Lam-Alef 压缩后的 shaped 偏移），
以及 run/装饰字形级 `pure_ltr_line_starts_one_subsequence_at_the_first_glyph`、
`pure_rtl_line_starts_one_subsequence_at_the_last_drawn_glyph`、
`cross_style_line_expands_at_each_subsequence_start`、
`interaction_only_style_change_starts_a_new_subsequence`（同时校验命中区 target 与绘制同序）、
`supplementary_pair_keeps_its_utf16_offsets_in_both_directions`、
`arabic_shaping_indexes_the_shaped_units`、`style_equal_component_parts_stay_one_subsequence`。

### 字形筛选与可点击区域

普通界面使用首字形；聊天输入 EditBox 使用 nonFishy 字体，补全列表仍使用普通字体
（`ChatScreen.init`）。`FontSet.selectProviders` 先筛选 options，再保留至少首供一个
codepoint 的 provider；完全遮盖的 provider 不可作为过滤后备。`FontSet.hasFishyAdvance`
保留首字形，并从 active providers 中选择首个 normal/bold advance 均不满足 `<0` 或 `>32`
的后备；无合格者使用 missing。普通文本继续保留负数、小数和大宽度。

筛选与混淆桶取烘焙前的 metrics。超大 bitmap cell 最终烘焙为 missing 时，不能用 missing
的宽度重算筛选或桶；`FontSet.getRandomGlyph` 按首供字形的 ceil-advance 桶随机选 codepoint，
返回该 codepoint 的 nonFishy 烘焙结果。`HudFontGlyphMap` 同时保存两路 glyph 与原始桶宽，
`non_fishy()` 和 `HudFontGlyphBook::non_fishy_glyphs` 提供显式查询。图集、普通查询、过滤查询
和混淆池共享同一 generation；克隆到 simulation 的测量投影保留两路数据并支付 retained memory。

聊天输入的 visible substring、display start、光标、选择区域和实际字形都使用同一过滤 map；
行内灰色 suggestion suffix 也使用过滤字体（`EditBox.extractWidgetRenderState`）。补全弹窗的条目、tooltip
和宽度保留普通字体，只有锚点所需的输入 prefix 宽度取过滤字体；命令方块及其他测量消费者
继续使用普通字体。字体重载同时替换渲染 book 和原生测量投影，不在 consumer 重新判断 fishy。

Vanilla 26.1 的交互命中由 `ActiveTextCollector.findElementUnderCursor` 先检查 prepared bounds
（`Font.PreparedTextBuilder.bounds` 只并集 drawable glyph 与 underline/strikethrough effect，按 floor/ceil
取整，纯空白行没有 bounds），再逆变换 pose 得到局部浮点坐标，对每个 `ActiveArea` 作左/上闭、
右/下开判断；`Font.PreparedTextBuilder.visit` 先访问 drawable glyph，再访问 empty area，后续命中覆盖前项。
无 bitmap 字形由 `EmptyArea`（`activeLeft`/`activeTop`/`activeRight`/`activeBottom`）给出 pen 起、bold-aware advance 宽的九像素区域，
有 bitmap 字形取 `BakedSheetGlyph`（`left`/`right`/`top`/`bottom`/`GlyphInstance.activeRight`）的 active 边界：斜体 shear、bold
extra thickness 与阴影偏移都在内，`activeRight` 仍是 `x + getAdvance(bold)`。
`ActiveTextCollector.ClickableStyleFinder` 在点击时按 `includeInsertions`
分两次扫描，所以 `ChatScreen.mouseClicked` / `handleComponentClicked` 的普通点击只认 click event、Shift 点击只插入
insertion，而 `BookViewScreen.visitText` 只认 click event 且分页指示文本不参与命中。

BBB 由 `hud_component_text_hit_geometry` 与绘制共用同一次 pen walk 提供这些事实：每个字形给
`HudGlyphArea`（含 `EmptyArea` 变体）与 prepared bounds，交互目标随源字符经过 bidi 重排与
Arabic shaping。chat 与 book 各自按 Vanilla 的可见行集合（`ChatComponent.forEachLine` 顶行先、
底行后；`BookViewScreen` 前十四行）与绘制原点（chat 为缩放 pose 下的 `(0, textTop)`；book 绝对坐标）
执行同一判定；两条 Vanilla 命中路径都没有 scissor（`Parameters.withScissor` 只服务滚动文本），
因此可见性由 prepared bounds 与可见页行集合承担。命中结果只在 generation/revision 仍匹配时
可执行，见[聊天编辑、命令语法与 UTF-16 文本](client-chat-and-command-text.md#chatscreen-文本与生命周期-owner)。

聊天页面选项取启动配置：`options.txt` 的 `chatScale`、`chatWidth`、`chatHeightFocused`、
`chatHeightUnfocused`、`chatLineSpacing`、`chatOpacity`，或对应的 `--chat-scale`、
`--chat-width`、`--chat-height-focused`、`--chat-height-unfocused`、
`--chat-line-spacing`、`--chat-opacity`（均为 `UnitDouble`，显式参数优先，越界行保留
初始值，未聚焦高度初始值为 `ChatComponent.defaultUnfocusedPct` 的 0.4375），经
`Renderer::set_hud_chat_options` 进入 renderer；分行宽度变化即按 `rescaleChat` 重新分行。
`hud::text_layout::chat_geometry` 的 `HudChatGeometry` 是唯一布局 owner：宽度取
`ChatComponent.getWidth` 的 `floor(pct * 280 + 40)`，分行宽度按 `ChatComponent.addMessageToDisplayQueue`
对 double 商取 floor，页宽与底边按 `ChatComponent.extractRenderState` 对 float 商取 ceil/floor；
行距给出 `(int)(9 * (spacing + 1))` 的 entry 高度与 `Math.round(8 * (spacing + 1) - 4 * spacing)`
的文字偏移（`ChatComponent.extractRenderState`），每页行数为聚焦/未聚焦高度
`floor(pct * 160 + 20)` 整除该行高（`ChatComponent.getHeight`）；文字、标签指示条用
`chatOpacity * 0.9 + 0.1`，队列文字再乘 0.5，行背景只读 `textBackgroundOpacity`
（`ChatComponent.extractRenderState`）。pose 为 `ChatComponent.extractRenderState` 的
`scale(s, s).translate(4, 0)`。行背景、queue/restricted 背景、tag 指示条、文字、modified 图标与
滚动条都以 Vanilla 局部整数坐标经该 pose 提交；点击采集复用绘制路径
（`ChatComponent.captureClickableText`），prepared bounds 经 `ScreenRectangle.transformMaxBounds` 变换后判定，
局部坐标按 JOML `Matrix3x2f` 的 float 求逆。消息行、队列行与限制提示按
`ChatComponent.extractRenderState` 的接受顺序逐字形命中（最后命中者胜），队列文字整段带展开点击；
tag 区域用 `ChatComponent.isMouseOver` 的局部矩形。滚动后的页行由 `HudChatGeometry::visible_rows`
（`ChatComponent.forEachLine`）同时供绘制与命中。

限制提示宽于页面时按 `ChatComponent.extractRenderState`、`ComponentRenderUtils.clipText` 与
`StringSplitter.headByWidth` / `WidthLimitedCharSink` 截到 `maxWidth - width("...")` 并追加无样式省略号；截断版
带 `chat_screen.restricted` 悬停提示，省略号不可点击。提示文字顶边为
`lineAboveMessagesY + entryHeight - entryBottomToMessageY - 1`。聚焦时绘制
`ChatComponent.extractRenderState` 的双列滚动条（Java int 运算、`y > 0` 时 alpha 170 否则 96），
左列在 `newMessageSinceScroll` 为真时取 `0xCC3333`、否则 `0x3333AA`（`ChatComponent.java:225`）。
modified 图标只由聚焦 access 绘制
（`ChatComponent.handleTagIcon`）：指针落在某条消息任一可见行的字形区域时画在其末行，落在某行
自身图标悬停框（比 sprite 下沿多一像素）时画在该行，位置取该行宽度加 4（`GuiMessage.Line.getTagIconLeft`）；
tag 提示先于文字悬停提示（`GuiGraphicsExtractor.setTooltipForNextFrameInternal` / `extractDeferredElements`）；文字悬停取接受顺序中
最后命中的 `ShowText`：截断的限制提示、队列行、消息行（`GuiGraphicsExtractor`）。
队列行样式带 `chat.queue.tooltip` 悬停（`ChatComponent.java:52-54`），native 把其本地化文本随
`HudChatContext::queue_tooltip_text` 投影，指针落在队列字形区域时由 `hud_chat_prompt_tooltip` 选出、
按 `max(guiWidth / 2, 200)` 分行绘制（`GuiGraphicsExtractor.componentHoverEffect`）。

滚动位置与 `newMessageSinceScroll` 由输入侧的 `HudChatScroll` 持有：renderer 每帧把当前分行数与
聚焦每页行数作为 `HudChatScrollPage` 反馈给 simulation（仅在变化时发送），滚轮
（`ChatScreen.mouseScrolled`）与 PageUp/PageDown（`ChatScreen.keyPressed`，步长为每页行数减一）
按 `ChatComponent.scrollChat` 在每次滚动时钳到 `lines - linesPerPage` 再钳到 0，到 0 时清除标志。
每次替换聊天源条目，renderer 以 `HudChatAppendedLines` 报告新增分行：替换前的分行数与序号超过旧最新
条目的每条消息在 100 行裁剪前的分行数；旧条目被清空（`ChatComponent.clearMessages`）时前置行数为 0，
world 原位替换删除标记（`ClientChatState::display_refresh_revision`，`ChatComponent.deleteMessageOrDelay`）
则按 `refreshTrimmedMessages` 以空表重报全部条目（`ChatComponent.refreshTrimmedMessages`）。runtime 在同一帧
的 `HudChatScrollPage` 之后把它作为 `SimulationCommand::HudChatAppendedLines` 送入 mailbox；仅在
ChatScreen 打开且位置大于 0 时，每个分行置标志并按加入前的行数 `scrollChat(1)`，每条消息后裁到
100 行（`ChatComponent.addMessageToDisplayQueue`）。ChatScreen 关闭，或被限制、对话框、命令确认屏幕替换时，
`resetChatScroll` 同时清零位置与标志（`ChatScreen.removed`、`ChatComponent.resetChatScroll`）。

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 位图与空白提供者 | `BitmapProvider`、`SpaceProvider` | `font::providers` + 共享图集 | `aligned` | 资源栈编解码、负步进、缺图回退、PNG 内容隔离与预算、活跃/`nonFishy` 选择测试 |
| 引用与选项过滤器 | `FontManager` 依赖与过滤器合并 | 字体提供者依赖图 | `aligned` | 整文件隔离、外层过滤覆盖、重复引用、失败传播与隐藏候选预算测试 |
| 字形选择与过滤 | `FontSet.selectProviders`/`computeGlyphInfo` | 字体选择与字形映射表 | `aligned` | 首供/活跃裁剪、普通/粗体过滤、缺失回退、原始度量与混淆/聊天输入测试 |
| TrueType 字体提供者 | `TrueTypeGlyphProviderDefinition`、`TrueTypeGlyphProvider`、`NativeImage.copyFromFont` | `font::ttf`（`TtfGlyphSource` 惰性 `ab_glyph` 光栅化）+ 共享图集 | `equivalent` | `font/` 前缀、charmap/skip、纯 Rust `ab_glyph` 轮廓度量与按需灰度光栅、oversample/shift、缺字、优先级、预算/重载与自校验 TTF 测试 |
| Unihex 解析、覆写与度量 | `UnihexProvider` | `font::unihex` | `aligned` | ZIP/HEX/边界/度量、完整码点 JSON 目录、活跃/`nonFishy` 与混淆重载测试 |
| 图集 UV 与字形样式 | `GlyphBitmap`、`FontSet` | 共享图集 + `hud_glyphs` | `aligned` | 原始像素与倒数缩放、有符号高度、Java 步进、浮点 `bearing_left`/`bearing_top`、UV 内缩与空白提交测试 |
| 有符号浮点度量与分行 | `Font.width`、`StringSplitter` | `hud_glyphs`、HUD/世界文本布局 | `aligned` | 有符号/小数步进、首个非零超宽字形、空格/换行分隔与源分段样式回归测试 |
| 二维字形点击区域 | `ActiveTextCollector`、`Font.PreparedText` | `text_layout`、`book` 共用 `hud_component_text_hit_geometry` | `equivalent` | 聊天/书本字形热区、空白行无包围盒、小数与负笔位置、斜体/粗体/阴影、空区域序、可见页裁剪与点击/插入两路扫描测试 |
| 双向重排与阿拉伯语塑形 | 语言与文本管线 | 客户端文本管线（`text_visual_order`） | `equivalent` | 字符、镜像与样式测试，以及 HUD/告示牌/TextDisplay/地图的事件身份、子序列局部索引与装饰线回归测试 |
| ICU 区域与时区格式化 | ICU 日期/时间格式化 | `bbb-client-world::local_time`（归属 `ui-local-time-icu`） | `equivalent` | `ui-local-time-icu` 专项测试 |

## 结构性不变量

- 过滤器必须在提供者可见性判定前应用；`reference` 展开不得打乱原有提供者顺序。
- 首供优先（first-provider-wins）不得受哈希表遍历顺序或图集装箱顺序影响。
- `unihex` 码点范围、行宽、ZIP 条目数、解压字节与图集尺寸均须受有界安全预算约束。
- 所有文本消费者共用同一字形度量表，严禁为 CJK 或特定字符集单独硬编码宽度。

## 聚焦测试（focused tests）和真实证据

focused tests 覆盖整文件 codec 隔离、filters、reference cycle/order、unihex digit widths、duplicate、override、
blank/bounds、metrics、oversample、UV、budget 和共享 stack priority/filter。
缺字页面与数字共享字体的测试覆盖 HUD/world text、数量字形几何与 reload rollback；
完整官方资源与 GPU 行为按[开发流程与测试](../../development/testing-and-gates.md)选择对应验证。
ttf provider 的 focused 证据是 codec/loader 单测
`fixture_font_is_a_checksummed_truetype_face`、`truetype_scaling_matches_the_fixture_metrics`、
`default_size_11_scales_outline_bounds_and_advance`、`oversample_scales_the_face_and_divides_every_metric`、
`shift_translates_the_outline_but_not_the_advance`、`skip_removes_exactly_the_named_codepoints`、
`rasterized_triangle_stores_grayscale_coverage_in_the_shared_atlas`、
`faces_without_glyf_or_a_unicode_charmap_are_rejected`，以及走真实 pack loader 的
`runtime_truetype_provider_loads_metrics_and_coverage_through_the_pack`、
`runtime_truetype_missing_codepoint_answers_vanillas_miss`、
`runtime_truetype_skip_removes_codepoints_before_selection`、
`runtime_truetype_precedence_follows_provider_order_and_first_wins`、
`runtime_truetype_reload_releases_the_previous_provider_page`、
`runtime_truetype_glyph_draws_with_its_own_origin_and_advance`；消费侧由 renderer 的
`ttf_shaped_glyph_bearing_flows_through_the_live_styled_text_pass` 固定负 bearing 的实际
quad 与 `activeRight`。提交的 fixture 是代码生成的 5 字形 TTF（`glyf`/`hmtx`/短 `loca`/
Unicode `cmap`，表校验和与 `head.checkSumAdjustment` 自洽），不依赖任何系统字体；
`BBB_TRUETYPE_FONT` 守卫的 `local_system_truetype_font_acceptance` 可对真实系统字体复跑，
真实 pack→atlas 测试在 10px 下逐像素核对 `B` 的 `(6,0,6)` metrics 与 6×6 灰度覆盖值，并断言由该字形生成的 styled quad。

二维点击区域的 focused 证据是 renderer 的
`chat_click_uses_glyph_areas_and_rejects_gaps_and_blank_lines`（字形内外、交互间隙、纯空白行无
bounds、上下留白）、`chat_click_keeps_fractional_pens_and_ignores_the_rounded_interval`
（2.5 宽字形与负 advance 的回折笔位置）、`chat_click_area_reflects_italic_bold_and_shadow_extents`
（斜体 shear、bold overhang、显式零阴影）、`chat_click_visits_empty_areas_after_every_glyph`、
`chat_click_clips_to_the_visible_page_and_drawn_rows`（可见页、行间留白与 scroll）与
`chat_click_separates_click_only_and_insertion_only_targets`（两路扫描与 tooltip hover）；
非默认缩放由 `chat_scale_sanitizes_like_unit_double_and_wraps_at_the_floored_width`、
`chat_scale_pose_matches_the_capture_path_float_rounding`（期望值取自 26.1 client jar 的
`ScreenRectangle` 与 JOML）、`scaled_chat_pointer_targets_follow_the_scaled_rows_through_scroll`、
GPU readback 的 `startup_chat_scale_rewraps_and_draws_the_rows_its_pointer_hits` 与启动侧
`stored_chat_scale_reaches_startup_and_explicit_cli_wins` 覆盖；其余页面选项由
`chat_options_sanitize_each_unit_double_to_its_initial_value`、
`chat_options_derive_the_page_from_vanilla_width_height_spacing_and_opacity`（期望值与 26.1 JDK 上
同一表达式的输出一致）、`chat_width_option_rewraps_messages_and_reserves_the_modified_icon`、
`chat_page_rows_and_scroll_clamp_to_the_option_lines_per_page`、
`focused_scrollbar_follows_the_vanilla_integer_thumb`（含新消息颜色）、
`new_lines_scroll_a_scrolled_page_one_line_at_a_time`、
`replaced_messages_report_the_lines_vanilla_appends`、
`modified_icon_shows_only_in_focused_chat_for_the_hovered_message_or_its_own_box`、
`restriction_prompt_clips_with_a_plain_ellipsis_and_hits_only_its_glyphs`、
`queue_hit_is_focused_revision_bound_and_limited_to_its_glyphs`、
`queue_tooltip_shows_over_the_queue_glyphs_of_focused_chat`、GPU readback 的
`startup_chat_page_options_draw_rows_opacity_scrollbar_and_hovered_icon`（含新消息颜色、队列提示与
新增分行报告）、启动侧 `stored_chat_page_options_reach_the_renderer_and_explicit_cli_wins`、
world 的 `delete_chat_replaces_visible_message_on_sixtieth_gui_tick`、runtime 的
`chat_render_cache_rebuilds_only_for_display_revision_or_level` 与走真实 `apply_commands` 的
`chat_wheel_and_page_keys_clamp_to_the_rendered_page_at_scroll_time`、
`new_chat_lines_follow_a_scrolled_focused_page_until_it_returns_to_the_bottom` 覆盖，
以及 `book_page_click_uses_glyph_areas_and_rejects_gaps_and_padding`、
`book_page_click_ignores_styles_without_a_click_event`；消费侧由
`shift_insertion_target_inserts_and_a_plain_click_falls_through`、
`message_click_precedes_edit_and_rejects_previous_screen_revision` 与书本
`lectern_component_click_keeps_page_server_owned_and_closes_before_command` 覆盖真实
MouseButton 分派与 page/insertion 动作。

## 当前差异

网络字符串由共享 UTF-8 扫描器按 Netty 替换语义与 UTF-16 单元数收界。有符号浮点分行、四类文本装饰线子序列身份与旧版 `§` 格式码的已解析/原始调用面均已完成生产接线：共享辅助入口接受已解析 run 列表，移动效果面板的名称、时长、测量与截断读取同一列表，输入框原始值保持未解析状态。

`BitmapProvider.Definition.load` 从完整图片计算整数网格与浮点缩放，`BitmapProvider.Glyph.bake`（`GlyphBitmap.getOversample`）以原始像素尺寸及倒数缩放提交；BBB 完整保留该算术并区分空白字形与零尺寸位图，聚焦测试覆盖 CPU 顶点、实例与提交序。二维点击区域按 prepared bounds 与字形有效边界接线；`Parameters.withScissor` 仅服务滚动文本容器，聊天与书本的 Vanilla 提取路径不传入该参数。

默认资产的 `bitmap`/`unihex` 测试与自校验纯 Rust `ab_glyph` TTF 测试分别覆盖对应提供者，完整官方资产与 GPU 读回按[开发流程与测试](../../development/testing-and-gates.md)执行。[客户端账本](../../../unsupported-client-features.md)当前无未关闭的 `# Todo` 缺口。

## 账本归属

本页对应 `ui-font-provider-pipeline`；新确认的生产差异与产品范围裁决统一登记于[客户端账本](../../../unsupported-client-features.md)。`bbb-client-world::local_time` 的 ICU 区域与时区格式化归属 `ui-local-time-icu`，不在本页范围。

## 精确重开条件

当目标版本变更字体提供者编解码、度量公式，或新增文本消费者绕过共享字形映射表时，须重新核验本机制。

## 相关页面和源码索引

- [聊天编辑、命令语法与 UTF-16 文本](client-chat-and-command-text.md)
- [离线资产可信链](../resources/client-offline-asset-trust.md)
- Vanilla：`FontManager`、`FontSet`、`FontOption`、`UnihexProvider`、`TrueTypeGlyphProvider`、`TrueTypeGlyphProviderDefinition`、`GlyphBitmap`、`FontTexture`
