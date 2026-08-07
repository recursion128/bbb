# 构建、target 与缓存

## 统一入口

开发和门禁使用 `scripts/cargo-dev.sh`。主工作树的稳定 target 是
`/tmp/bbb-target-main`；worker 只有在 `AGENTS.md` 的并行合同满足时才使用独立
`/tmp/bbb-target-<domain>`。repo-local `target/` 不生成、不提交。

日常 focused test 优先 `scripts/cargo-dev.sh fast-test` 或
`scripts/cargo-dev.sh test -p <crate> <filter>`；helper 设置 opt-level-0 深栈测试需要的
`RUST_MIN_STACK`。deny-warnings 已在 workspace lints 中，禁止临时切换
`RUSTFLAGS='-D warnings'` 导致共享 cache 全量失效。

Python 开发工具由仓库根的 `.python-version`、`pyproject.toml` 与 `uv.lock` 共同
锁定；`.venv/` 是 ignored 的工作树本地 cache。`scripts/bootstrap` 是唯一允许下载
Python 解释器和 Python/Rust 依赖的初始化入口；Rust 只预取本机 target 与既有
`aarch64-apple-darwin` cross-check target，不把非目标 Windows 依赖混入 cache 合同。
它默认也准备锁定的 Vanilla 26.1 workspace。日常命令统一经过 `scripts/python`，
使用 exact uv-managed interpreter、
`--offline --locked --exact`，因此不会在 lint、测试、生成器或门禁中隐式联网、换
解释器、改锁或保留未声明包。`scripts/vanilla-source-tools.sh` 只作为旧生成物中命令
注记的兼容入口，不拥有第二套环境。

## 缓存与清理

稳定 target 以 slice/domain 复用，不在每轮删除。`scripts/cargo-dev.sh prune` 默认 dry
run，用 keep list 清理真正陈旧 cache。sccache、linker 或 profile 改动必须先记录可复现
基线（host、commit、cold/warm、command、wall time、target size）再采用。

Vanilla JAR、libraries、assets 与反编译源码使用独立的平台 cache，既不放进 target，
也不进入 Git；位置、完整性和按版本清理由
[Vanilla workspace](vanilla-workspace.md)统一管理。

## 检查点（checkpoint）

`gate` 对最终 HEAD 运行完整检查并写 last-gated stamp。高风险工程按 `goal.md` 的阶段
checkpoint 执行，不靠频繁清空 cache 获得表面一致。需要重新评估 sccache、linker、
profile 或 cache 策略时，用上一节的同机 cold/warm 合同生成外部原始产物，并把采用后的
长期规则更新在本页；旧测量由 Git history 保留，不在受维护文档中复制快照表。
