# Vanilla workspace 与供应链

## 所有权与目录

bbb 提交工具、锁、repo-native 派生 catalog/evidence 和小型 fixture；官方 JAR、
libraries、assets 与反编译源码永远只存在于本地 cache。最终工具树是：

```text
scripts/
  vanilla                       # 稳定 CLI，只有 prepare 可联网
  dev                           # doctor/lint/test/generate
  process                       # POSIX process-group supervisor
  input                         # macOS dev / Linux release input adapter
  bbb_tools/
    vanilla/                    # workspace/lock/download/prepare/verify/clean
    execution/                  # monotonic deadline 与进程监管
    host/                       # capability facts 与 profile policy
    input/                      # backend-neutral contract 与平台 adapter
  data/vanilla-26.1.lock.json
  tests/
```

顶层 `scripts/vanilla` 必须是可执行文件，因此实现 package 使用
`scripts/bbb_tools/vanilla/`，不能同时创建同名目录。`VanillaWorkspace` 是唯一 cache
路径 owner；Python/Rust generator、docs lint、benchmark、测试与产品启动只能接收显式
root 或调用它对应的 resolver，不得从 `$HOME` 拼路径。

## 命令与网络边界

```sh
scripts/vanilla prepare 26.1
scripts/vanilla prepare 26.1 --offline
scripts/vanilla verify 26.1
scripts/vanilla path 26.1 --kind sources
scripts/vanilla clean 26.1
```

只有 `prepare` 可以导入下载模块并访问网络。普通 build、gate、pytest、generator 与
benchmark 在 cache 缺失时失败，并给出完整的 `scripts/vanilla prepare 26.1` 命令。
`prepare --emit-lock-candidate <file>` 是在线发现动作；审查 candidate 后才用
`scripts/vanilla lock apply 26.1 --candidate <file>` 更新提交锁。普通 prepare 不读取
“latest”，也不改锁。

root 优先级只有 `--vanilla-root`、`BBB_VANILLA_ROOT`、平台默认值三层。macOS 默认
`~/Library/Caches/bbb/vanilla`；Linux 默认
`${XDG_CACHE_HOME:-~/.cache}/bbb/vanilla`。旧 repo 和旧环境变量不是 fallback。
cache 若位于 bbb worktree 或本身是 Git repo，prepare/clean 会拒绝。

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

| 能力 | macOS 开发 | Linux 开发 | Linux release |
| --- | --- | --- | --- |
| Vanilla prepare/verify/generator | 是 | 是 | 是 |
| Ruff/pytest/full gate | 是 | 是 | 是 |
| 本机 input smoke | CoreGraphics；需 Accessibility 授权 | `ydotoold` 可选 | 不作 release 证据 |
| 真实 seat/display/IME receipt | 否 | 否 | 持久 uinput → libinput → sway → `wl_seat` |
| Vulkan/Wayland release acceptance | 否 | 否 | 是 |

`scripts/dev doctor --profile portable|mac-dev|mac-input|linux-release` 把 host facts 与
profile policy 分开报告。libei 是成熟的 portal/remote-desktop 注入协议，适合桌面授权
会话，但当前 release 合同还要求持久物理扫描码、相对指针、seat capability 和 compositor
display/IME 控制，因此不把“检测到 libei”误当作 release receipt。Linux release 使用
repo-owned `linux-release` backend；macOS 只用于开发 smoke，不声称替代 Linux 验收。

## 验证、迁移结果与回滚点

核心验证命令：

```sh
scripts/dev lint python
scripts/dev test python --suite all
scripts/dev generate --check all
scripts/vanilla verify 26.1
scripts/cargo-dev.sh gate
scripts/playable-gate.sh fixture
scripts/platform-acceptance-gate.sh       # 仅声明的 Linux release host
```

迁移已按可回滚 checkpoint 完成：先建立新 resolver，随后导入 prepare/verify；在 Linux
实际生成并校验，在 macOS 合同/能力测试中验证平台路径；再用
`verify --compare-root <旧根>` 对旧、新 `sources/26.1` 做内容树 hash 比对，结果相同后批量
迁移消费者和文档。每个阶段的回滚点都是“旧消费者尚未删除、旧 repo 未修改”的提交边界。
当前代码不再读取旧路径或旧环境变量；独立旧 repo 未被删除、未改写历史，可由用户在确认
双平台实机与适用 live/platform gate 后自行清理，不属于 bbb 工具的 destructive action。
