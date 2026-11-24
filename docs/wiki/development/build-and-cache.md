# 构建、target 与缓存

## 跨平台统一入口

日常开发、worker 隔离、Web 打包与完整门禁在 Linux、macOS、Windows 上统一使用 `cargo dev` 与
`cargo worker`（由零外部依赖的 `crates/bbb-xtask` 提供；由于 Cargo `[alias]` 不经过 shell
且不展开 `%TEMP%`/`$TMPDIR` 等环境变量，xtask 自身构建隔离在 gitignored 的 `.local/xtask-target`，
`.cargo/config.toml`、`scripts/bbb_tools/python_runner.py` 与 CI 缓存共用该路径）。

主工作树的稳定默认 target 在 Linux/macOS 为 `/tmp/bbb-target-main`，在 Windows 为
`%TEMP%\bbb-target-main`；worker lane 从固定的暖池 `<temp>/bbb-target-lane-<N>`
（`BBB_TARGET_POOL_SIZE` 个槽，默认 8）租一个槽，租约记录在 `<git-common-dir>/bbb/target-leases.tsv`，
由 `cargo worker create/cleanup/release` 管理。新目录会失去已有编译产物，因此不按 lane 名新建冷 target；
**严禁生成或提交 repo-local `target/`**。

可将 `BBB_TARGET_ROOT` 设为指定磁盘上的绝对目录，统一存放主树、worker lane、batch gate、WASM、
wasm-bindgen CLI 安装目录（`bbb-wasm-bindgen-*`）及编译缓存。显式 `CARGO_TARGET_DIR`、`BBB_TARGET_POOL_ROOT`、`BBB_WASM_TARGET_DIR`
仍优先用于对应构建。

`cargo dev` 自己解析要驱动的 `cargo`：依次取 `CARGO`、`PATH` 上的 `cargo`、最后
`CARGO_HOME/bin`（或 `~/.cargo/bin`、`%USERPROFILE%\.cargo\bin`），并把解析出的工具链目录前置到
`PATH`，因此没有 rustup 目录的裸 shell（git hook、后台 agent 会话）同样可用；三处都没有时指名
`cargo` 与预期路径报错退出。

## Profile 与测试筛选

日常 focused test 使用已热的 profile，入口是 `cargo dev fast-test` 或
`cargo dev test -p <crate> <filter>`；`.cargo/config.toml` 与 `cargo dev` 自动注入深栈测试所需的
`RUST_MIN_STACK=16777216` 以及已准备的 `BBB_VANILLA_ROOT`。已有热池未构建 `fast-test` 时，切换
profile 会先支付另一套冷编译，不能未经测量统一切换。包含真实 noise 生成、完整传送门半径或雷击世界场景时，
优先保留 `test` 的优化构建；`fast-test` 的 opt-level-0 会放大这些场景的运行时间，编译快不等于反馈快。

按测试完整路径或 owner 模块限定 filter，避免 `portal`、`save` 等宽匹配带入其他 owner 的昂贵场景。
遇到 profile 引起的超时，保留失败记录，仅用适合的 profile 重跑超时项和新增改动；不提高全局 timeout，
也不重跑已通过且未受后续改动影响的测试。测试持续占满 CPU 时，先取超时线程的调用栈，区分编译成本与真实运行时热点，
修复生产查询路径后用相同输入比较结果和耗时，不通过禁用模拟来提速。`Cargo.toml` 的 workspace lints 已默认启用 `warnings = "deny"`；**禁止临时设置
`RUSTFLAGS='-D warnings'` 或其它覆盖 flag**，以免使共享编译缓存指纹全量失效并触发重编译。

## Worker 工作树与 Python 环境

worker 的标准 focused-command 路径是：

```sh
cargo worker run <lane> -- cargo dev test -p <crate> <filter>
```

`run` 在执行命令前一次性验证现有 worktree、branch、暖池租约和主工作树已准备的 Python / Vanilla 环境，
再进入 worker 并导出 `BBB_CARGO_TARGET_NAME`、`CARGO_TARGET_DIR`、`BBB_WASM_TARGET_DIR`、`BBB_WEB_OUT_DIR` 与 `BBB_VANILLA_ROOT`；验证失败
不会分配新租约或启动用户命令。`env` 与 `shell-env` 仅供交互 shell 只读查询已有 worktree 与租约（不分配或改写租约）。

Python 开发工具由仓库根的 `.python-version`、`pyproject.toml` 与 `uv.lock` 共同锁定；`.venv/` 是
ignored 的主工作树本地 cache（Unix 为 `.venv/bin/python`，Windows 为 `.venv\Scripts\python.exe`）。
`cargo dev bootstrap` 是唯一允许下载 Python 解释器和 Python/Rust/Vanilla 依赖的初始化入口：Rust 预取本机
target 及第二平台 cross-check target（macOS 为 `x86_64-unknown-linux-gnu`，其余平台为 `aarch64-apple-darwin`；不需要跨平台检查时可加 `--skip-cross-target`），并默认准备锁定的 Vanilla 26.1 workspace。
Cargo metadata 生成器必须传当前 rustc host 的 `--filter-platform`，避免跨平台无关 target 依赖阻断。

日常 Python 命令（`cargo dev python`、`cargo dev py-test`、`cargo dev py-lint`、`cargo dev docs-lint`、
`cargo dev audit-p0` 以及 POSIX live 脚本调用的 `scripts/python`）统一解析主工作树的已准备 `.venv`
（有 `uv` 时使用 `uv run --offline --locked --exact --no-python-downloads`），不会在 lint、测试、生成器或门禁中隐式联网、
换解释器或改锁。POSIX 进程监管入口 `scripts/process` 是唯一直接 `exec` 主工作树 `.venv` 解释器的脚本，
以保持调用 PID、最终进程 PID 与 process group identity 一致。worker 不创建、复制或软链 `.venv`；
worker 内的 `cargo dev`、`scripts/python` 和 `scripts/process` 都通过 Git worktree 元数据复用主工作树 `.venv`，
同时保留当前 worker 的 cwd 与 `PYTHONPATH=<worker>/scripts`。

日常 Python 定向测试使用 `cargo dev py-test scripts/tests/test_docs_lint.py`，可接多个文件或
pytest node id，以 `-k '<expression>'` 进一步筛选；路径相对当前 worktree 根目录解析。
有路径或 `-k` 时默认 4 个 pytest worker，`--jobs 0` 用于串行调试，`--jobs N` 或 `--jobs auto`
显式选择并行数；full gate 用 `--suite all` 跑整套。

## 缓存清理

稳定 target 按角色长期复用，不在每轮删除：

- `cargo dev size`：查看当前构建缓存根目录下各 `bbb-target-*` 的体积。
- `cargo dev prune`：默认 dry-run，保留 `main`、`playable`、`soak`、`wasm`、`wasm-bindgen-cli` 与全部 `lane-<N>` / `wasm-lane-<N>` 暖池槽，
  将其余陈旧 `bbb-target-*` 及误生成的 repo-local `target/` 列为删除候选；确认后加 `--force` 清理。
- `cargo dev clean-target <name>`：按名称安全删除单个 `bbb-target-<name>` 目录。
- WASM Web 客户端构建与打包默认落于共享构建缓存根目录（`BBB_TARGET_ROOT`，未设置时 Linux/macOS 为 `/tmp`、Windows 为 `%TEMP%`）下的
  `bbb-target-wasm`、`bbb-web-dist` 与 `bbb-web-cache`；设置非 `main` 的 `BBB_CARGO_TARGET_NAME` 时自动追加 `-<name>` 后缀，
  显式 `BBB_WASM_TARGET_DIR`、`BBB_WEB_OUT_DIR`、`BBB_WEB_CACHE_DIR` 优先。

暖 target 不保证零重编译：同一树换到另一个 worktree 绝对路径或切换 feature 集合时，Cargo 可能重建受影响依赖链；
跨 crate owner family 迭代时尽量固定 `-p`/features/profile，通过缩小 test filter 提速。
需要重新评估 linker、profile 或缓存策略时，在同一 host、commit、profile 与命令上记录可复现的 cold/warm 基线再采用。

Vanilla JAR、libraries、assets 与反编译源码使用独立的平台 cache，既不放进 target，也不进入 Git；
位置、完整性和按版本清理由 [Vanilla workspace](vanilla-workspace.md) 统一管理。

full gate 不构建正式 release，也不启动 platform live；这些步骤只在发布前按
[开发流程](testing-and-gates.md#live-与发布级验收)运行。
