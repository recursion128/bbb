# 目标：bbb 与 Minecraft Java 26.1 彻底对齐

## 总目标

让 `bbb` 渲染/世界/协议子系统与 Minecraft Java **26.1** 完全对齐：
`docs/unsupported-features.md` 中，除刻意不支持的 feature 外，其余全部彻底支持。
以**每 slice 一个可合并提交**的方式推进，直到清单清空。

## 硬性约束（每 slice 必须满足）

- **严格按 vanilla 源码转写**：权威源 `~/Work/mc-code/sources/26.1/`，落地前核对确切常量/索引/公式，不臆测。
- **合并门禁（每 slice 全绿）**：
  - `bbb-renderer` / `bbb-world` / `bbb-pack` 各 **0 warning**；
  - native warning **≤ 14**（当前 13）；
  - `cargo fmt --all --check` exit 0；
  - `cargo test --workspace` 全绿。
- **`bbb-renderer` 不得依赖 `bbb-pack`**。
- 每 slice 更新 `docs/unsupported-features.md` + memory（`~/.claude/projects/-home-zgy-Work-bbb/memory/`）。
- 提交前 `rm -f crates/bbb-native/src/lib.rs`（native 是二进制，靠 `touch main.rs` 触发 warning 重算）。
- 始终用中文汇报。

## 已完成（截至 2026-06-25）

实体基础贴图全部接好；近期完成的子特征 / 整条脉络见
memory `entity-texture-path-status.md`。其中**马类（equine）贴图脉络已彻底打通**：
骷髅马/僵尸马、活体马（7 种毛色 + markings 叠层）、成年与幼年驴/骡，
全部走 textured 路径并带 vanilla 步态/头部/尾巴姿态（幼驴因 `setupAnim` 锁 xRot=-30° 而静态）。
村民 / 僵尸村民的类型、职业、等级徽章叠层已完成；猪、adult equine
（马/驴/骡/骷髅马/僵尸马）、adult strider、adult camel/camel_husk、adult living nautilus
和 zombie nautilus 的鞍装备层、llama decor、nautilus body armor、horse/zombie-horse
body armor 装备层已完成，baby 按 vanilla 行为跳过没有 baby model 的装备层。
实体附着方块模型已开始打通：snow golem carved pumpkin head block layer 已完成。

## 剩余大子系统（按优先级）

1. **实体上的方块模型渲染器**
   末影人搬方块 / 铁傀儡花 / 哞菇蘑菇 / 铜傀儡风化 —— 继续扩展 block-model-on-entity 用户。
2. **实体上的物品渲染器**
   手持物 / 狐狸叼物 / 物品展示框内容 —— 需要 item-model primitive。
3. **世界侧动画计时器**
   狼湿身着色、溺尸/海豚游泳重姿态等 client-tick 动画。

> 落地前务必先在 bbb 里 grep 确认该 feature 确实缺失（历史上多次「以为缺失实则已实现」）。
> 索引/数据陷阱见 memory `entity-metadata-index-layout.md`；模型/代理历史见 `proxy-entity-replacement.md`。
