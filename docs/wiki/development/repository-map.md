# 仓库地图

## 产品 crate

| 路径 | semantic owner | 主要输出 |
| --- | --- | --- |
| `crates/bbb-protocol` | 26.1 wire codec、固定/生成协议事实 | typed packets、ids、render facts |
| `crates/bbb-net` | socket、连接 phase、收发顺序 | ordered network events |
| `crates/bbb-world` | canonical client world、entity、chunk、UI state | query/state/effects inputs |
| `crates/bbb-pack` | pack stack、assets、resource generation | validated resources |
| `crates/bbb-item-model` | item/font/model resource 解析与 bake | model/glyph catalogs |
| `crates/bbb-render-types` | 跨 native/renderer 的窄渲染类型 | frame primitives |
| `crates/bbb-renderer` | GPU backend、terrain/entity/UI passes | presented frame/counters |
| `crates/bbb-audio` | audio backend、voice 生命周期 | playback observations |
| `crates/bbb-control` | 外部只读 snapshot 与有界控制请求 | control API |
| `crates/bbb-platform-macos` | macOS 系统调用（Dock 图标）；闭包保持可 cross-check | AppKit 调用 |
| `crates/bbb-native` | CLI、platform、simulation、跨边界 wiring | 可执行客户端 |

依赖方向遵循[总体架构](../architecture/overview.md)：底层 crate 不反向读取 runtime 或
renderer 状态；跨 crate API 表达语义，不为拆文件暴露 broad `pub`。

## 工具与生成事实

`scripts/` 保存 source audit、coverage、benchmark、门禁和文档 lint。稳定入口是
`scripts/dev`、`scripts/vanilla`、`scripts/process` 与 `scripts/input`；实现归
`scripts/bbb_tools` 下按 vanilla、execution、host、input 分域，测试归 `scripts/tests`，锁归
`scripts/data`。顶层入口不拥有业务逻辑；
`scripts/fixtures/` 保存小型、可评审且无网络的 fixture。`docs/evidence/vanilla-26.1/*.json` 保存 26.1
coverage/generated catalog，不保存外部 jar/assets/capture。`tests/` 和各 crate 内
`#[cfg(test)]` 按 owner 放置 focused tests。

## 文档

[`docs/README.md`](../../README.md)是总入口，Wiki 保存长期事实。`goal.md`、ledger 与 Git 的
权威边界见[文档工作流](documentation-workflow.md)。全部 Markdown 处理结论见
[inventory](document-inventory.md)。
