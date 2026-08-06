# 字体与文本：provider、glyph metrics 与 atlas

机制 ID：`ui-font-provider-pipeline`

## 机制目的和边界

该机制从资源包 font JSON 解析 `bitmap`、`space`、`reference`、`unihex` 和 filter，按
Vanilla provider 优先级生成 glyph metrics/bitmap/atlas，供 HUD、screen、map 和 entity
text 使用。Unicode bidi/shaping 已有独立文本处理；ICU locale/timezone 日期格式属于
另一机制 `ui-local-time-icu`，由 [parity matrix](../../parity/vanilla-26.1-matrix.md) 的该行
拥有，不在本页边界内。

## Vanilla 26.1 权威实现

`FontManager` 收集 builder，解析 reference dependency，把外层与被引用 provider filter
合并。`FontSet.selectProviders` 先按 `FontOption.Filter` 选择；
`computeGlyphInfo` 按 provider 顺序取首个 glyph，并另选 non-fishy glyph。

`UnihexProvider` 遍历 ZIP 中 `.hex` entry，解析 4/5/6 位 codepoint 与 32/64/96/128 个
bitmap hex digits（8/16/24/32 bit width）；相同 codepoint 后读覆盖先读。定义中的
`size_overrides` 按列表顺序先从自动 map 移除并固定 bounds，余下 glyph 扫描非零列。
glyph 高 16、oversample 2，advance 为宽度/2+1；bold/shadow offset 为 0.5。
`GlyphBitmap` 被 `FontSet` stitch 到 atlas，边缘留采样 padding。

## 决策输入与状态所有权

统一 `PackResourceStack` 决定 source/indexed/selected/server 的 font JSON/ZIP/PNG 字节和
顺序；font options 决定 filter；provider graph
决定候选顺序；glyph provider 拥有 bitmap/metrics，resource generation 拥有 atlas，
文本 layout 拥有本次 codepoint/style 序列。

## 判断与处理顺序

1. 合并 pack 后解析 font definitions，验证资源 id、provider 数量与循环。
2. 在 reference 展开时合并 filters，并按选项排除 provider。
3. 对每个 codepoint 执行 first-provider-wins；缺字使用 missing glyph。
4. unihex 先处理 ordered overrides，再扫描自动 bounds。
5. 以 glyph top/ascent/advance/oversample stitch atlas；layout 与所有文本消费者共享 metrics。

## 生命周期和 fallback

font catalog 随 resource generation 原子替换。provider/ZIP/PNG 越界或损坏受 resource
budget 限制并失败，不允许无界解压；单个 codepoint 无 provider 时确定性使用 missing
glyph。默认 options 为 `uniform=false`、`jp=false`。

## BBB 实现映射

`bbb-item-model/src/font/providers.rs` 解析 provider/reference/filter；
`font/unihex.rs` 有界读取 ZIP/HEX 并生成 glyph；font atlas runtime 合并 bitmap/unihex；
`bbb-render-types/src/hud_glyphs.rs` 保留 UV、advance、top、style quads；
`bbb-native`/`bbb-renderer` 的 HUD、screen、map、sign、TextDisplay 等消费同一
`HudFontGlyphMap`。
definition 与 unihex ZIP 不再经过字体专用 indexed fallback；它们和 language/sound 一样
只使用默认 `vanilla` pack 所在的共享 stack。

### ChatScreen 文本与生命周期 owner

`ui-chat-screen-surface` 的唯一可变 owner 是 simulation `ClientInputState`：它持有
MESSAGE/COMMAND、256 UTF-16 edit value、recent≤100、latest draft、history buffer/position、
exit reason 与 restriction previous-screen state。Full/System/Hidden 权限只从同 generation
`ChatPresentationConfig.visibility` 查询；普通 component click 重新解析 canonical click event，
Shift click 只插入 inherited `Style.insertion`，两者都校验 level/delivery/resource/screen revision。

renderer 只接收 bounded rich runs、draft style、restricted prompt/screen 和 hit target，不持有
click/insertion payload；control `input.key`/`input.text` 重放同一 key/IME handler，并从 immutable
simulation generation 发布当前 edit value 与 scalar history/draft/permission 状态。screen narration
复用统一 delayed `ScreenNarrationRuntime`。RestrictionsScreen 返回时恢复原 ChatScreen；其他
screen replacement 以 INTERRUPTED 收口，成功提交以 DONE 收口，失败提交保留当前 generation。
playable live gate 禁止 `net.send_chat_*` 捷径，并分别验证 Full 的 draft/history/message/command
roundtrip 与 System 的 blocked-message/allowed-command machine status。

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| bitmap/space provider | provider definitions | `font/providers.rs` + atlas | `equivalent` | provider/atlas tests |
| reference/filter | `FontManager` dependency/filter merge | provider graph | `equivalent` | filter/reference fixtures |
| first-provider-wins | `FontSet.computeGlyphInfo` | glyph map insertion | `equivalent` | provider ordering tests |
| unihex parse/override/metrics | `UnihexProvider` | `font/unihex.rs` | `equivalent` | ZIP/HEX/bounds/metrics tests |
| atlas UV/style | `GlyphBitmap`, `FontSet` | atlas + `hud_glyphs` | `equivalent` | UV/top/bold/shadow tests |
| bidi/shaping | language/text pipeline | native text pipeline | `equivalent` | native text-layout focused tests |
| ICU locale/timezone parity | ICU-backed date/time | local time formatter（属 `ui-local-time-icu`） | `equivalent` | [matrix 行](../../parity/vanilla-26.1-matrix.md) |

## 结构性不变量

- filter 在 provider 可见性决定前应用；reference 不能改变原顺序。
- first-provider-wins 不得被 HashMap iteration 或 atlas packing 顺序改变。
- unihex codepoint、line width、ZIP entries、expanded bytes 与 atlas size 都有上限。
- 所有消费者使用同一 glyph metrics；不能为 CJK 单独硬编码宽度。
- ChatScreen renderer/control 不得恢复独立 edit、permission、click 或 recent/draft owner。

## 聚焦测试（focused tests）和真实证据

focused tests 覆盖 filters、reference cycle/order、unihex digit widths、duplicate、override、
blank/bounds、metrics、oversample、UV、budget、共享 stack priority/filter，以及反向 source
guard 禁止 index-specific resolver。真实完整 26.1 objects 构建 8192×4729 atlas、
114432 个 default glyph；`中`、`日`、`한`、U+20000 锚点验证资源链、
layout 与 GPU sampling。

## 当前差异

当前 consumed 26.1 font assets 已覆盖，本机制无未关闭差异。

## 账本/goal 路由

本页当前无账本条目。`local_time` 的 ICU/locale/timezone parity 由 `ui-local-time-icu`
matrix 行拥有，其重开条件写在该行，不在本页。

## 精确重开条件

目标版本改变 font provider codec/metrics，或新增文本消费者绕过共享 glyph map 时重开
本机制。

## 相关页面和源码索引

- [离线资产可信链](../resources/offline-asset-trust.md)
- Vanilla：`FontManager`、`FontSet`、`FontOption`、`UnihexProvider`、`GlyphBitmap`、`FontTexture`
