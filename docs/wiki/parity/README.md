# Vanilla 26.1 机制覆盖审计

[全局矩阵](vanilla-26.1-matrix.md)按可验证机制而不是“大功能”记录 owner、required
projection 和 evidence。状态由机器报告约束，不能只表示 Wiki 审计置信度：

- `equivalent`：当前 focused contract、owner dependency hash、正式 receipt 和 BBB evidence 已对应；
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
scenario 反向列出覆盖的 Mechanism ID。报告按 `implementation`→`checkpoint`→
`code-freeze`→`evidence-sweep`→`final-performance`→`final` 排序；`partial` 的开放原因、
BBB owner/path 与 focused-test hash、capture contract/binary identity 都由 generator 派生。
`implementation-gap` 或 `focused-evidence-pending` 仍存在时，generator/lint 强制
`implementation_ready=false`、`code_freeze_ready=false`，禁止提前进入正式 sweep。
Rust focused contract 必须是可解析到真实 `#[test]` 或 `#[tokio::test]` 的精确
`path#symbol`；普通 async helper 和 source-string guard 不能冒充可执行证据。
`scripts/python python scripts/parity_acceptance.py plan --base-commit <commit>` 给出 changed-path→Mechanism ID→
validation class，不相关修改不会在开发阶段作废别的 owner receipt；中央 owner 或采集输入变化
仍精确失效其依赖闭包。full gate 只证明整仓回归，不隐式启动 live/visual/RenderDoc/ABBA。

`scripts/python python scripts/parity_acceptance.py check` 是 portable 结构/source-hash lint；
正式交付追加 `--require-artifacts --require-complete --require-final-head`，在最终 clean、full-gated
HEAD 重新核对全部依赖、formal artifact 和阶段顺序。diagnostic smoke 只能定位，不能满足
正式 receipt。最终 sealing commit 可更新 `docs/**` 与 `goal.md` 的状态/证据 metadata，但
evidence commit 到最终 HEAD 间不得有任何 code/tool/fixture 变化，最终 HEAD 自身必须有 clean
full-gate log receipt；`evidence-sweep` 冻结的 matrix owner、required projection、route、inventory
owner/source、not-needed/deferred scope ruling 与 validation class 语义 hash 必须和 final 相同。
报告发现 false `equivalent` 时必须先降级并路由，不能放宽 checker。

正式 ABBA 的每个 Vanilla/BBB run 还必须携带同一个已提交、clean、stage 为
`evidence-sweep` 且 consistency gate 通过的 acceptance receipt identity；summary 和 lint
同时核对 receipt commit/blob hash，不能在 ABBA 跑完后补写阶段文字冒充前置门禁。
