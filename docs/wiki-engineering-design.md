# 中文 Vanilla 26.1 机制等价与 BBB Wiki 工程合同

## 目标与完成标准

本高风险 docs/workflow engineering slice 建立 repo-owned 中文 Wiki，并把它作为
Vanilla 26.1 客户端机制、BBB 语义所有者（semantic owner）、状态投影
（projection）、消费者、测试与验证证据的 canonical 知识入口。工程同时审计全部
受维护 Markdown，消除双重权威、断链和孤儿页面，并建立机制级 parity matrix。

完成必须同时满足：`docs/README.md` 与 `docs/wiki/README.md` 导航完整；总体架构、
offline runtime 和六个首批纵向领域形成机制级对照；每个受维护 Markdown 均进入
迁移清单；旧页面有明确保留/迁移/重定向/archive/删除结论；链接、机制 id、parity
状态、ledger/goal route、repo-local path 与孤儿规则可机械检查；focused fixtures、
per-commit checks 和最终 full gate 全部通过。

## 范围与非目标

本工程只拥有 `docs/**/*.md`、仓库文档入口、`scripts/docs-lint.sh` 及其 focused
fixture tests。允许为迁移更新 `README.md`、`AGENTS.md`、`goal.md` 和
`docs/unsupported-features.md` 中的文档链接、canonical route 与当前 slice 状态。

本工程不改任何产品 crate、协议、renderer、world、资源加载或运行时行为；不提交
Vanilla 源码、官方客户端、assets、benchmark 原始产物或开发机绝对路径依赖。审计
发现的产品 `missing` / `partial` / `unverified` 只能写入 parity matrix，并按真实
状态路由到 `docs/unsupported-features.md` / `goal.md`，不得顺手实现。

## 权威边界与迁移规则

- `AGENTS.md` 继续唯一拥有 agent 执行和协作规则。
- `goal.md` 继续唯一拥有当前优先级、active slice、ready contract 和门禁路由。
- `docs/unsupported-features.md` 继续唯一拥有未关闭缺口状态。
- `docs/goal-archive.md` 与 Git history 继续唯一拥有完成索引和实现历史。
- Wiki canonical 页面拥有长期架构、机制、术语、开发说明和验证方法；不得复制一份
  可独立漂移的 active/deferred ledger 或提交史。
- 每个现有 Markdown 必须在 inventory 中记录当前职责、准确性、canonical
  destination、处理方式、引用者、内容类别及冲突。内容被完整吸收并更新引用后，
  旧页才可改成短重定向、archive 或删除。
- Wiki 叙述性标题、正文、表格说明、图注、导航和术语解释使用简体中文；类型、方法、
  packet、registry、resource location、路径、命令和测试名保留原文。
- 本地 `~/Work/mc-code/sources/26.1/` 只作为写作时的权威输入。lint 在该源码树存在时
  可以加强验证，不存在时必须明确降级，不能让 CI 依赖开发机绝对路径。

## Vanilla 证据合同

每个机制页记录 Vanilla owner 类/方法、决策输入、查询坐标或状态来源、判断顺序、
fallback、生命周期/阶段、相邻调用关系，以及 BBB owner、类型、数据流、消费者和
focused evidence。正文采用中文机制描述与精确定位，不复制大段官方源码，也不以
“看起来相似”代替决策级说明。

## 串行阶段

1. **Phase A**：Markdown inventory、引用/冲突审计和 canonical ownership 合同。
2. **Phase B**：Wiki taxonomy、双入口、导航、术语表和机制页模板。
3. **Phase C**：总体架构、offline runtime 与 protocol→world→renderer-input 主链。
4. **Phase D**：离线资产可信链、核心状态流、方块渲染、fluid、字体文本、官方同载
   性能比较六个纵向领域及 Vanilla/BBB 对照。
5. **Phase E**：全局 Vanilla 26.1 parity matrix；未关闭项只链接权威 ledger。
6. **Phase F**：旧文档迁移、重定向/archive/删除、引用更新和双 canonical 清理。
7. **Phase G**：增强 docs lint，并添加通过、断链、重复 id、非法状态、孤儿页和失效
   route focused fixtures。
8. **Phase H**：完整 diff review、per-commit checks、提交、full gate 与 stamp 判定。

阶段失败一律 fix-forward；内容未迁移前不删除旧页，broken link、孤儿 canonical
页面或双 owner 未清零前不进入交付。

## Focused tests 与 checkpoint

Phase A～F 持续运行 `scripts/docs-lint.sh`、Markdown 相对链接/导航检查、inventory
覆盖和 repo-local path 检查。Phase G 的 focused command 为
`scripts/vanilla-source-tools.sh python -m unittest scripts.tests.test_docs_lint`，并由
新增 fixtures 覆盖通过、断链、重复 mechanism id、非法 parity 状态、孤儿页和失效
ledger/goal route。每次提交前还运行 `cargo fmt --all --check`、`git diff --check`
和完整 diff review。

开工基线 `5b515fff3da4fbf68fef90a098f3f75e3d8123b8` 已通过 full gate 与 live playable。
Phase D 首批机制纵向闭环和 Phase G lint/迁移闭环分别是高风险 checkpoint；最终
HEAD 必须再次运行 `scripts/cargo-dev.sh gate`。本 slice 不改变可玩行为，因此
playable gate 默认不重跑，只验证 `last-playable-live` 仍是最终 HEAD 的祖先；若迁移
意外触及产品行为则合同失效并停止。

## 安全停止点

只有阶段产物自洽、inventory 无漏项、已创建页面全部可达、相对链接与 route 检查
通过且工作树状态可完整报告时，才能在外部 blocker 下停止。Phase D 未完成六个纵向
领域、Phase F 尚有双 canonical owner、或 Phase G lint 可绕过失败页面时，都不是
安全停止点。
