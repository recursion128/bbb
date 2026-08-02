# Vanilla 26.1 机制覆盖审计

[全局矩阵](vanilla-26.1-matrix.md)按可验证机制而不是“大功能”记录 owner、required
projection 和 evidence。状态只表示 Wiki 审计置信度：

- `equivalent`：决策级机制和 BBB evidence 已对应；
- `partial`：当前表面存在，但机制覆盖不完整；
- `missing`：Vanilla 当前客户端行为没有 BBB owner；
- `active`：已有唯一 active 工程 route；
- `deferred-external`：被精确外部条件阻塞；
- `not-needed`：目标客户端无该 surface 或 Vanilla 无客户端行为；
- `unverified`：尚未完成决策级取证。

`active`、`deferred-external`、`missing` 和未关闭 `partial` 必须链接
`docs/unsupported-features.md` 或 `goal.md`；矩阵不能单独改变调度。完成项链接机制页、
focused test 或 archive evidence。
