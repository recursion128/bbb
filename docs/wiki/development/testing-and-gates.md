# 测试、门禁与证据

## 证据层级

1. Vanilla source evidence：确认机制，不证明 BBB wiring。
2. unit/focused test：证明一个 owner 的输入、顺序、fallback 和错误边界。
3. integration test：证明 projection/consumer 使用同一事实。
4. real evidence：真实 assets/server/GPU/window/audio 等只在需要时验收。
5. full/playable gate：证明整仓或声明可玩表面没有回归。

不得用截图“看起来接近”替代机制断言，也不得用 unit test 声称真实平台已通过。

## 工程切片门禁

`goal.md` 是 checkpoint/full-gate 唯一排程 owner。每次 commit 前执行其 per-commit
checks，包括 focused tests、docs lint、format、diff check 和完整 diff review。
`scripts/cargo-dev.sh gate` 在 `/tmp/bbb-target-main` 运行完整门禁，并把通过 commit 写入
`.git/bbb/last-gated`；`batch-gate` 只用于失败定位。

playable gate 需要真实 live session 时才运行并更新 `.git/bbb/last-playable-live`。
纯 docs/workflow slice 不改变可玩行为，可验证旧 stamp 是最终 HEAD 祖先；如果修改产品
行为，这个豁免立即失效。

## 文档测试

`scripts/docs-lint.sh` 是入口；Wiki 检查由其调用的 repo-owned helper 完成。fixture
必须无网络且覆盖通过、断链、重复 mechanism id、非法 parity 状态、孤儿 canonical
页面与失效 goal/ledger route。不能通过排除目录或放宽规则绕过失败内容。
