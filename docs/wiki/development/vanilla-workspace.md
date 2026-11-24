# Vanilla workspace 与供应链

## 所有权与目录

bbb 提交工具、锁、repo-native 派生 catalog/evidence 和小型 fixture；官方 JAR、
libraries、assets 与反编译源码永远只存在于本地 cache。工具树结构为：

```text
crates/bbb-xtask/               # 跨平台统一入口 (cargo dev / cargo worker / cargo web-up)
scripts/
  python                        # POSIX live 脚本共用的根 uv 离线锁定 runner
  vanilla                       # POSIX 兼容 CLI，转发至 bbb_tools.vanilla.cli
  dev                           # POSIX 兼容 CLI，转发至 bbb_tools.dev_cli
  process                       # POSIX process-group supervisor
  input                         # macOS / Linux release input adapter
  audit_p0.py                   # 跨平台 P0 allowlist 分类审计
  docs_lint.py                  # 跨平台文档、ledger、生成事实与单文件行数检查
  cross_check.py                # 第二平台类型检查
  bbb_tools/
    python_runner.py            # 跨平台主工作树 .venv / uv 解析
    vanilla/                    # workspace/lock/download/prepare/verify/clean
    execution/                  # monotonic deadline 与进程监管
    host/                       # capability facts 与 profile policy
    input/                      # backend-neutral contract 与平台 adapter
  data/vanilla-26.1.lock.json
  tests/
```

顶层 `scripts/vanilla` 在 POSIX 下为可执行文件，因此实现 package 使用
`scripts/bbb_tools/vanilla/`，不能同时创建同名目录。`VanillaWorkspace` 是唯一 cache
路径 owner；Python/Rust generator、docs lint、benchmark、测试与产品启动只能接收显式
root 或调用它对应的 resolver，不得从 `$HOME` 拼路径。

## 命令与网络边界

新工作树通常运行 `cargo dev bootstrap`：它先安装根 `.python-version` 指定的
uv-managed Python、同步 `uv.lock` 与 `Cargo.lock`，再进入 Vanilla prepare。
`--skip-vanilla` 只准备工具链；`--offline` 要求所有层的 cache 已存在。

```sh
cargo dev bootstrap
cargo dev vanilla prepare 26.1
cargo dev vanilla prepare 26.1 --offline
cargo dev vanilla verify 26.1
cargo dev vanilla path 26.1 --kind sources
cargo dev vanilla clean 26.1
```

初始化阶段只有 `cargo dev bootstrap` 的 uv/Cargo 同步及其调用的 `prepare` 可以访问网络。
普通 build、gate、pytest、generator 与 benchmark 在 cache 缺失时失败，并给出明确的
`cargo dev bootstrap` 或 `cargo dev vanilla prepare 26.1` 命令。
`prepare --emit-lock-candidate <file>` 是在线发现动作；审查 candidate 后才用
`cargo dev vanilla lock apply 26.1 --candidate <file>` 更新提交锁。普通 prepare 不读取
“latest”，也不改锁。

root 优先级只有 `--vanilla-root`、`BBB_VANILLA_ROOT`、平台默认值三层：

- Windows 默认：`%LOCALAPPDATA%\bbb\vanilla`
- macOS 默认：`~/Library/Caches/bbb/vanilla`
- Linux 默认：`${XDG_CACHE_HOME:-~/.cache}/bbb/vanilla`

旧 repo 和旧环境变量不是后备路径。cache 若位于 bbb worktree 或本身是 Git repo，
prepare/clean 会拒绝。

## 锁、布局与失败语义

提交锁固定 version metadata、client/server JAR、library path/URL/digest、asset index、
Vineflower 版本/URL/digest、Java major 和完整反编译参数。cache 初始布局为：

```text
<root>/downloads/26.1/
<root>/sources/26.1/
<root>/sources/26.1/assets/minecraft/
<root>/state/26.1.json
```

prepare 对所有下载校验 size、SHA-1 和锁中存在的 SHA-256；按版本加进程锁；在同一
cache 的隐藏 staging 中完成下载、反编译和 tree hash 后，以 `os.replace()` 发布，state
最后出现。失败 staging 会清理，原 workspace 会恢复。state 记录 lock digest、host、
Java、Vineflower 参数、cache hit/download 计数和源码树 identity。verify 重算全部 JAR、
library、4,749 个 asset object 与源码树；clean 只删除一个明确版本，也能回收首次 prepare
中断后没有 state 的 staging。下载、摘要、并发锁、Java major、反编译与 unsafe clean
使用不同错误类别和退出码。

## 平台能力与输入边界

| 能力 | Windows x86_64 | macOS aarch64 | Linux 开发 | Linux x86_64 release |
| --- | --- | --- | --- | --- |
| Vanilla prepare/verify/generator | 是 | 是 | 是 | 是 |
| Ruff/pytest/full gate | 是 | 是 | 是 | 是 |
| 可归因平台 input | 原生主机手工验收 | Quartz + Accessibility | `ydotoold` 可选 smoke | 持久 uinput → libinput → sway → `wl_seat` |
| 平台专有额外 receipt | DX12/WASAPI/MP4 | Cocoa/Metal/OpenAL/CoreAudio | 否 | Wayland DPI/fullscreen + fcitx5 IME |
| 自动化 release acceptance | 否（见 ledger） | 是（无常驻 runner） | 否 | 是 |

`cargo dev doctor --profile portable|mac-dev|mac-input|mac-release|linux-release` 把 host facts 与
profile policy 分开报告。libei 是成熟的 portal/remote-desktop 注入协议，适合桌面授权
会话，但当前 release 合同还要求持久物理扫描码、相对指针、seat capability 和 compositor
display/IME 控制，因此不把“检测到 libei”误当作 release receipt。Linux release 使用
repo-owned `linux-release` backend；macOS release 使用 repo-owned `macos-quartz` backend，
且不声称替代 Linux 的 compositor/display/IME 验收。

## 验证与变更合同

核心验证命令：

```sh
cargo dev py-lint
cargo dev py-test --suite all
cargo dev generate --check all
cargo dev vanilla verify 26.1
cargo dev gate
scripts/playable-gate.sh fixture
scripts/platform-acceptance-gate.sh       # 声明的 Linux x86_64 或 macOS aarch64 host
```

所有消费者只通过本页所述 resolver 与 `cargo dev vanilla`（或 POSIX `scripts/vanilla`）访问 workspace；
禁止恢复个人绝对路径、旧环境变量或仓外源码猜测。需要比较迁移输入时使用
`verify --compare-root <reference-root>` 做内容树 hash 比对。
