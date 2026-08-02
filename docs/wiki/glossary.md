# 术语表

| 术语 | 中文解释 |
| --- | --- |
| semantic owner | 语义所有者：负责某类事实生命周期、有效性和变更规则的唯一边界。 |
| projection | 状态投影：从权威状态提取给下游消费者的有界、只读表示。 |
| fallback | 回退行为：输入缺失、无效或 provider 不可用时的确定性结果。 |
| first-provider-wins | 首个 provider 优先：按资源包合并后的顺序选择第一个能提供结果的 provider。 |
| resident workload | 驻留工作负载：客户端已保留、可能被后续帧使用的 chunk、section、纹理等集合。 |
| visible workload | 可见工作负载：当前相机与剔除规则后实际进入提取或绘制的集合。 |
| renderer input | 渲染器输入：world/runtime 发布给 renderer 的一帧一致性投影，不是第二份世界权威状态。 |
| canonical state | 权威状态：收到 packet 或本地预测后可持续查询、参与后续决策的状态。 |
| transient event | 瞬时事件：按顺序消费一次、不会伪装成持久状态的效果。 |
| backend state | 后端状态：GPU、音频设备、窗口等实现资源，只由对应运行时拥有。 |
| focused test | 聚焦测试：覆盖一个机制合同或失败边界的最小相关测试集合。 |
| full gate | 完整门禁：`scripts/cargo-dev.sh gate` 定义的全仓检查，并记录通过的 commit。 |
| playable gate | 可玩门禁：真实连接、资源加载、输入、画面和退出生命周期的有序现场验收。 |
| mechanism id | 机制标识符：parity matrix 中稳定、唯一的 ASCII kebab-case 键。 |
| reopen condition | 重开条件：能机械判断外部条件或目标表面何时变化的精确条件。 |
