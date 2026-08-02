# 架构与代码组织规则

## 语义优先

模块按 semantic owner 命名和拆分；wire、world、renderer-input 与 backend 类型不混为
一层。新增功能先确认 Vanilla owner 和所需 state，再选择现有 crate/module。不要用
catch-all `lib.rs`/`main.rs` 积累跨域逻辑，也不要为了行数创建单函数模块。

## 拆分条件

小于约 1000 行的 cohesive 文件默认保持完整。拆分必须同时有命名明确的 owner、窄
API、focused test 或明确测试计划，以及可陈述的依赖/编译/理解收益。单 caller helper
留在 caller 附近；Rust 使用 2018 布局 `foo.rs` + `foo/bar.rs`，不新增 `foo/mod.rs`。

## API 和状态

- 优先 structured packet/state type，不跨边界传 ad-hoc bytes/string。
- `pub` 只暴露消费者真正需要的最小类型与查询；测试不要迫使生产 API 变宽。
- derived snapshot 不回写 canonical state；backend handle 不进入 world。
- Vanilla 行为有本地源码时必须注明类/方法，并复刻输入、坐标、顺序与 fallback。
- renderer 不能用 layer/name 推导独立 block-state facts。

## 重构与验证

结构重构不能顺带改变功能合同。先保留行为测试，再移动 owner，最后删除旧路径。新增
module 的测试与 owner 同位置；跨层只添加能证明 integration contract 的测试。完整
执行流程以 `AGENTS.md`、`goal.md` 与[测试门禁](testing-and-gates.md)为准。
