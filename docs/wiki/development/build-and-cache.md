# 构建、target 与缓存

## 统一入口

开发和门禁使用 `scripts/cargo-dev.sh`。主工作树的稳定 target 是
`/tmp/bbb-target-main`；worker 只有在 `AGENTS.md` 的并行合同满足时才使用独立
`/tmp/bbb-target-<domain>`。repo-local `target/` 不生成、不提交。

日常 focused test 优先 `scripts/cargo-dev.sh fast-test` 或
`scripts/cargo-dev.sh test -p <crate> <filter>`；helper 设置 opt-level-0 深栈测试需要的
`RUST_MIN_STACK`。deny-warnings 已在 workspace lints 中，禁止临时切换
`RUSTFLAGS='-D warnings'` 导致共享 cache 全量失效。

## 缓存与清理

稳定 target 以 slice/domain 复用，不在每轮删除。`scripts/cargo-dev.sh prune` 默认 dry
run，用 keep list 清理真正陈旧 cache。sccache、linker 或 profile 改动必须先记录可复现
基线（host、commit、cold/warm、command、wall time、target size）再采用。

## 检查点（checkpoint）

`gate` 对最终 HEAD 运行完整检查并写 last-gated stamp。高风险工程按 `goal.md` 的阶段
checkpoint 执行，不靠频繁清空 cache 获得表面一致。历史 build measurement 保留在
[`cargo-build-performance.md`](../../cargo-build-performance.md)作为专门证据；本页是
当前长期规则的 canonical owner。
