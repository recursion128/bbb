# Vanilla 工作区与供应链

## 所有权与目录

bbb 提交工具、锁文件、仓库原生派生事实目录与证据文件以及小型测试夹具；官方 JAR、
依赖库、资产与反编译源码永远只存在于本地缓存。工具树结构为：

```text
crates/bbb-xtask/               # 跨平台统一入口 (cargo dev / cargo worker / cargo build-web / cargo web-up)
scripts/
  python                        # POSIX 真机脚本共用的根 uv 离线锁定执行器
  vanilla                       # POSIX 兼容命令行，转发至 bbb_tools.vanilla.cli
  dev                           # POSIX 兼容命令行，转发至 bbb_tools.dev_cli
  process                       # POSIX 进程组监管器
  input                         # macOS / Linux 发布输入适配器
  audit_p0.py                   # 跨平台 P0 白名单分类审计
  docs_lint.py                  # 跨平台文档、账本、生成事实与单文件行数检查
  cross_check.py                # 第二平台类型检查
  bbb_tools/
    dev_cli.py                  # 开发工具统一子命令分发
    anvil.py                    # 只读 Anvil 区域方块解码
    python_runner.py            # 跨平台主工作树 .venv / uv 解析
    vanilla/                    # 工作区、锁、下载、准备、校验与清理
    execution/                  # 单调截止时间与进程监管
    host/                       # 主机能力事实与策略配置
    input/                      # 后端无关输入合同与平台适配器
  data/vanilla-26.1.lock.json
  tests/
```

顶层 `scripts/vanilla` 在 POSIX 下为可执行文件，因此实现包使用
`scripts/bbb_tools/vanilla/`，不能同时创建同名目录。`VanillaWorkspace` 是唯一缓存
路径所有者；Python/Rust 生成器、文档检查、性能基准、测试与产品启动只能接收显式
根目录或调用其对应的路径解析器，不得从 `$HOME` 拼路径。

## 命令与网络边界

新工作树通常运行 `cargo dev bootstrap`：它先安装根 `.python-version` 指定的
由 `uv` 托管的 Python、同步 `uv.lock` 与 `Cargo.lock`，再进入 Vanilla 准备流程。
`--skip-vanilla` 只准备工具链；`--skip-cross-target` 跳过第二平台交叉检查目标；`--offline` 要求所有层的缓存均已存在。

```sh
cargo dev bootstrap
cargo dev vanilla prepare 26.1
cargo dev vanilla prepare 26.1 --offline
cargo dev vanilla verify 26.1
cargo dev vanilla path 26.1 --kind sources
cargo dev vanilla clean 26.1
```

初始化阶段只有 `cargo dev bootstrap` 的 `uv`/Cargo 同步及其调用的 `prepare` 可以访问网络。
普通构建、门禁、pytest、生成器与性能基准在缓存缺失时失败，并给出明确的
`cargo dev bootstrap` 或 `cargo dev vanilla prepare 26.1` 命令。
`prepare --emit-lock-candidate <file>` 是在线发现动作；审查候选锁后才用
`cargo dev vanilla lock apply 26.1 --candidate <file>` 更新提交锁。普通 `prepare` 不读取
“latest”，也不改锁。

根目录优先级只有 `--vanilla-root`、`BBB_VANILLA_ROOT`、平台默认值三层：

- Windows 默认：`%LOCALAPPDATA%\bbb\vanilla`
- macOS 默认：`~/Library/Caches/bbb/vanilla`
- Linux 默认：`${XDG_CACHE_HOME:-~/.cache}/bbb/vanilla`

旧仓库和旧环境变量不是后备路径。缓存若位于 bbb 工作树内或本身是 Git 仓库，
`prepare`/`clean` 会拒绝。

## 锁、布局与失败语义

提交锁固定版本元数据、客户端/服务端 JAR、依赖库路径/URL/摘要、资产索引、
Vineflower 版本/URL/摘要、Java 主版本号和完整反编译参数。缓存布局为：

```text
<root>/downloads/26.1/
<root>/sources/26.1/
<root>/sources/26.1/assets/minecraft/
<root>/state/26.1.json
<root>/state/26.1.verify-stamp.json
```

`prepare` 对所有下载校验文件大小、SHA-1 和锁中存在的 SHA-256；按版本加进程锁；在同一
缓存的隐藏暂存目录（staging）中完成下载、反编译和目录树哈希后，以 `os.replace()` 发布，`state`
最后出现。失败的暂存目录会清理，原工作区会恢复。`state` 记录锁摘要、宿主机、
Java、Vineflower 参数、缓存命中/下载计数和源码树标识。`verify` 默认执行快速元数据戳校验
（`<root>/state/26.1.verify-stamp.json`，核验文件大小、修改时间及锁与状态摘要），仅在首次校验、
元数据不匹配、指定 `--compare-root` 或显式传入 `--full` 时重算全部 JAR、依赖库、4,749 个资产对象与源码树的逐字节哈希；
`clean` 只删除一个明确版本，也能回收首次 `prepare` 中断后没有 `state` 的暂存目录。下载、摘要、并发锁、Java 主版本、反编译与不安全清理
使用不同错误类别和退出码。

## 平台能力与输入边界

| 能力 | Windows x86_64 | macOS aarch64 | Linux 开发 | Linux x86_64 发布行 |
| --- | --- | --- | --- | --- |
| Vanilla 准备/校验/生成器 | 是 | 是 | 是 | 是 |
| Ruff/pytest/完整门禁 | 是 | 是 | 是 | 是 |
| 可归因平台输入 | 原生主机手工验收 | Quartz + 辅助功能 | `ydotoold` 可选冒烟测试 | 持久 `uinput` → libinput → sway → `wl_seat` |
| 平台专有额外收据 | DX12/WASAPI/MP4 | Cocoa/Metal/OpenAL/CoreAudio | 否 | Wayland DPI/全屏 + fcitx5 输入法 |
| 自动化发布验收 | 否（见账本） | 是（无常驻运行器） | 否 | 是 |

`cargo dev doctor --profile portable|mac-dev|mac-input|mac-release|linux-release` 将主机能力事实与
策略配置分开报告。libei 是成熟的 portal/远程桌面注入协议，适合桌面授权
会话，但当前发布合同还要求持久物理扫描码、相对指针、输入座席（seat）能力和合成器
显示与输入法控制，因此不把“检测到 libei”误当作发布验收收据。Linux 发布行使用
仓库自有的 `linux-release` 后端；macOS 发布行使用仓库自有的 `macos-quartz` 后端，
且不声称替代 Linux 的合成器、显示与输入法验收。

## 验证与变更合同

核心验证命令：

```sh
cargo dev py-lint
cargo dev py-test --suite all
cargo dev generate --check all
cargo dev vanilla verify 26.1
cargo dev gate
scripts/playable-gate.sh fixture
scripts/platform-acceptance-gate.sh       # 声明的 Linux x86_64 或 macOS aarch64 主机
```

所有消费者只通过本页所述路径解析器与 `cargo dev vanilla`（或 POSIX `scripts/vanilla`）访问工作区；
禁止恢复个人绝对路径、旧环境变量或仓外源码猜测。需要比较迁移输入时使用
`verify --compare-root <reference-root>` 做内容树哈希比对。
