# Vanilla 26.1 unihex / CJK 字体闭环

## Goal and evidence

本工程将 Vanilla 26.1 `unihex` provider 接入 HUD、screen、sign、display、
name-tag 与 map text 共用的 immutable font generation。asset index `30` 中
`font/include/unifont.json`、`unifont.zip`、`unifont_jp.zip` 和
`unifont_pua.zip` 的声明大小与 SHA-1 均已验证；外部 asset 未进入仓库。

权威行为来自 `~/Work/mc-code/sources/26.1/` 中 `FontManager` 的
provider/filter/reference 顺序、`FontSet.computeGlyphInfo`，以及
`UnihexProvider` 的 ZIP 迭代、HEX 语法、size override、自动边界、位图上传与
glyph metrics。

## Frozen boundary

- `bbb-pack` 拥有离线 version metadata、asset index 与 object store 解析，校验
  声明大小和 SHA-1，并保证 selected pack 优先于 indexed Vanilla fallback。
- `bbb-item-model::font` 拥有 provider filter、有界 ZIP/HEX 解析、override 顺序、
  glyph packing 和共享 immutable atlas/book。
- `bbb-render-types` 拥有逐 glyph render scale 与 bold/shadow metrics；现有
  renderer/native text consumer 继续复用同一 glyph book。
- 边界不包含网络下载、launcher 修改、提交 Mojang asset、通用 text shaping、
  ICU/locale、新字体 UI 或无关资源泛化。

## Serial phases and tests

1. indexed Vanilla font resource 从本地 `26.1.json` 和 asset-index metadata
   解析；错误大小/SHA-1、畸形 hash 和缺失 object 会失败，selected pack 继续优先。
2. 条件 `unihex` definition 与有界 ZIP/HEX reader 覆盖 4/5/6 位 codepoint、
   8/16/24/32-bit row、duplicate overwrite、非法输入、archive budget、自动边界
   和 ordered override。
3. 每个唯一 provider 只进入共享 atlas 一次；Vanilla 2× oversampling、0.5
   bold/shadow offset 和 first-provider-wins 均保留。中文、日文、韩文和 non-BMP
   glyph 通过公共 renderer geometry。
4. 真实 26.1 对象生成 `8192×4492`、`114432` 个 default glyph 的 atlas；真实
   Wayland/Vulkan 启动完成上传。离屏 CJK 锚点保存在仓库外
   `/tmp/bbb-unihex-cjk-anchor.png`，SHA-256 为
   `f02ae81aea721033280542a6d8c4035b3ccb631c23e9bc3a5b36ce8826fc8da8`。

起点 full-gate 与 live-playable stamp 均为
`078d8c6ddc351d0e64a8c0cb34c1f37bf729163b`。工程按单一 slice fix-forward；
最终 focused tests、per-commit checks、full gate 与适用 live playable 结果由本次
提交及 gate stamp 记录。
