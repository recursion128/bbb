# bbb

`bbb` 是 Minecraft Java 26.1 的 repo-native 客户端，拥有独立的 protocol、world、
renderer、control 与 runtime 边界，不包装 Bevy、Azalea 或其他客户端栈。

当前 native crates：

- `bbb-native`
- `bbb-protocol`
- `bbb-net`
- `bbb-world`
- `bbb-pack`
- `bbb-renderer`
- `bbb-control`
- `bbb-item-model`

文档从 [`docs/README.md`](docs/README.md)进入；长期机制与架构见
[`docs/wiki/README.md`](docs/wiki/README.md)，当前路线和缺口分别见
[`goal.md`](goal.md)与
[`docs/unsupported-features.md`](docs/unsupported-features.md)。
