# 离线启动与 asset-index/object-store 可信链

机制 ID：`resources-offline-asset-trust`

## 机制目的和边界

该链证明官方客户端和 BBB 读取的是同一组 26.1 资源，并允许缺少普通 assets 目录时由
官方 asset-index/object-store 提供资源。它不下载资源，也不把外部对象纳入仓库。

## Vanilla 26.1 权威实现

`Main.main` 将 `--assetsDir` 与 `--assetIndex` 放入 `GameConfig.FolderData`；
`Minecraft` 创建资源管理器时消费该配置。版本 metadata 的 downloads/libraries 和
asset index 的 object hash 是启动前信任根。资源包层随后按 pack priority 构造
`MultiPackResourceManager`；资源查询必须遵守 namespace、relative path 与 stack 顺序。

## 决策输入与状态所有权

输入是 `26.1.json`、client/server jar、library/native metadata、index `30` 与每个
object 的 SHA-1/size。外部缓存拥有字节；launcher 只验证并生成 launch plan。
`bbb-pack` 拥有 pack roots、asset-index fallback 和已解码 resource generation。

## 判断与处理顺序

1. 校验 metadata 与所有声明文件，再创建进程。
2. native classifier 逐 entry 做路径安全检查，再提取到独立目录。
3. pack lookup 先按当前 pack stack 查普通 root；对 assets logical path 再查已验证 index
   hash 对象，不能把 index 当任意文件系统 escape。
4. provider graph、纹理、字体、声音等在同一 generation 内解析；成功后原子发布。

## 生命周期和 fallback

启动校验 fail closed。资源 reload 的新 generation 构建失败时，运行时保留上一代；单个
可选资源的 missing/invalid fallback 由其 provider 合同决定并保留诊断。object-store
是读取 fallback，不改变资源包 precedence。

## BBB 实现映射

官方 launcher 与验证器位于 `scripts/official_client_benchmark.py`。`bbb-pack` 的
asset-index/root lookup 为 item model、字体、纹理和音频提供字节；
`ClientResourceGeneration` 是跨消费者的一代资源 owner。解码预算和 path validation
仍由 pack safety 层约束。

## Vanilla/BBB 对照

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| version/library/native 校验 | launcher metadata contracts | `scripts/official_client_benchmark.py` | `equivalent` | metadata/classpath/native focused tests |
| native 路径安全 | trusted launcher extraction | benchmark safe extractor | `equivalent` | traversal/symlink/collision fixtures |
| logical asset → SHA-1 object | asset index/object store | `bbb-pack` asset-index fallback | `equivalent` | full 26.1 asset audit |
| pack precedence | `MultiPackResourceManager` stack | `bbb-pack` provider stack | `equivalent` | pack-stack focused tests |
| reload 原子性 | reload prepare/apply | `ClientResourceGeneration` | `equivalent` | generation failure/recovery tests |

## 结构性不变量

- hash/size 错误不得以 warning 后继续。
- path normalization 后仍不得越出目标 root。
- asset-index fallback 不得覆盖更高优先级 pack provider。
- generation 内消费者不能混用两次 reload 的资源。

## 聚焦测试（focused tests）和真实证据

`scripts.tests.test_official_client_benchmark` 覆盖 metadata、SHA-1/size、native extraction
和 offline CLI。`bbb-pack` focused tests 覆盖 pack stack、budget、path 与 object lookup。
完整 assets 曾真实生成 8192×4492 字体 atlas 和 114432 个 default glyph，证明 unihex
不是 fixture-only 路径；外部对象与图像不提交。

## 当前差异

本机制已覆盖当前 26.1 离线比较与运行资源面。未来新版本若改变 metadata、asset index
或 pack lookup，必须重新审计，不能沿用 26.1 hash。

## 账本/goal 路由

无未关闭条目。

## 精确重开条件

升级目标版本，或代码新增绕过 `bbb-pack` 的资源消费者时重开审计。

## 相关页面和源码索引

- [官方离线运行体系](../../architecture/offline-runtime.md)
- [字体与文本](../ui/font-and-text.md)
- Vanilla：`Main`、`Minecraft`、resource manager/pack classes
- BBB：`bbb-pack`、`bbb-item-model`、`scripts/official_client_benchmark.py`
