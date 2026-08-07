# Vanilla 26.1 机制覆盖审计

[全局矩阵](vanilla-26.1-matrix.md)按可验证机制而不是“大功能”记录 owner、required
projection 和 evidence。状态由机器报告约束，不能只表示 Wiki 审计置信度：

- `equivalent`：决策级机制、当前 gate/differential receipt 和 BBB evidence 已对应；
- `partial`：当前表面存在，但机制覆盖不完整；
- `missing`：Vanilla 当前客户端行为没有 BBB owner；
- `active`：已有唯一 active 工程 route；
- `deferred-external`：被精确外部条件阻塞；
- `not-needed`：目标客户端无该 surface 或 Vanilla 无客户端行为；
- `unverified`：尚未完成决策级取证。

`active`、`deferred-external`、`missing` 和未关闭 `partial` 必须链接
`docs/unsupported-features.md` 或 `goal.md`；矩阵不能单独改变调度。完成项链接机制页、
focused test、生成 coverage、checkout-local real evidence 或 Git history。

[parity acceptance report](../../evidence/vanilla-26.1/parity-acceptance.json)从矩阵、
owner inventory、ledger、full-gate 与外部 receipts 派生 147 条记录；每个 artifact/
scenario 反向列出覆盖的 Mechanism ID。`scripts/parity_acceptance.py check` 是 portable
结构/source-hash lint，正式交付再加 `--require-artifacts --require-complete` 校验本机
raw artifact 仍存在、hash 未变且声明范围没有开放项。报告发现 false `equivalent` 时
必须先降级并路由，不能放宽 checker。
