# 官方 Vanilla 26.1 离线运行体系

## 目的与信任根

官方离线客户端不是单一 flag。可复现运行由版本 metadata、jar、library/native、
asset index、SHA-1 object store、独立 `gameDir`、固定 options、离线身份、离线服务端
和固定世界共同构成；任一输入变化都会改变可比较的运行。

## 文件与校验链

1. 读取 `26.1.json`，校验 version id、main class、client download 与 libraries。
2. 按 metadata 的 SHA-1/size 校验 client jar、每个 library 和 Linux native classifier。
3. 提取 native 前拒绝绝对路径、`..`、反斜杠、NUL、symlink 和 basename 冲突。
4. 读取 asset index `30`；逐个将 logical asset 映射到
   `objects/<sha1[0:2]>/<sha1>`，校验每个对象的 size 与 SHA-1。
5. 在独立临时 `gameDir` 写固定 `options.txt`，外部输入和原始产物不进 Git。

上述 repo-owned launcher 是 `scripts/official_client_benchmark.py`，不是通用 Minecraft
launcher；它只服务于验证和等负载比较。

## Vanilla 26.1 身份与连接证据

本地源码的 `net.minecraft.client.main.Main` 注册 `--offlineDeveloperMode` 与
`--quickPlayMultiplayer`。`--accessToken` 语法仍必填；未传 `--uuid` 时调用
`UUIDUtil.createOfflinePlayerUUID(username)`。`Minecraft` 在该模式选择
`YggdrasilAuthenticationService.createOffline`、`UserApiService.OFFLINE`、
`UserApiService.OFFLINE_PROPERTIES` 和 `ProfileKeyPairManager.EMPTY_KEY_MANAGER`，也不
异步获取在线 profile。

`ClientHandshakePacketListenerImpl.authenticateServer` 的 `joinServer` 属于服务端请求
在线认证的路径。因此基准 server 固定 `online-mode=false` 和
`enforce-secure-profile=false`，使用固定用户名 `bbb-benchmark` 与非秘密占位 token
`offline`，绝不读取或记录真实凭据。

## 固定运行面

官方命令包含 `--version 26.1`、`--assetsDir`、`--assetIndex 30`、
`--width 1280 --height 720` 和
`--quickPlayMultiplayer 127.0.0.1:<port>`。server world 使用固定 seed、预生成 chunk、
只读 source snapshot、固定时间/天气/难度和资源包 SHA-1；运行副本可以变更，source
snapshot 不变。

server-authoritative replay 由 datapack tick function 每 tick 执行绝对 `tp`。
`TeleportCommand.performTeleport` 经 `ServerPlayer.teleport` 和
`ServerGamePacketListenerImpl.teleport` 产生 `ClientboundPlayerPositionPacket`；Vanilla
由 `ClientPacketListener.handleMovePlayer` 消费。两客户端因此接收同一位置、yaw、pitch
序列，而不是分别接受鼠标脚本。

## 生命周期与 fallback

所有校验先于进程创建；缺文件、hash/size 错误、危险 native entry 或 option 回读不符
都 fail closed。客户端顺序运行，process group 有界清理。资源、世界、工具和 capture
放在 `/tmp` 或用户指定外部目录；Wiki 只记录合同和摘要身份。

## 对照与证据

| 机制行为 | Vanilla 26.1 | BBB owner/path | 状态 | 验证证据 |
| --- | --- | --- | --- | --- |
| 离线 UUID | `Main.main` → `UUIDUtil.createOfflinePlayerUUID` | benchmark 固定用户名 | `equivalent` | launcher CLI tests |
| 离线服务 | `Minecraft` offline branches | official launcher contract | `equivalent` | real launch smoke |
| asset object store | `AssetIndex`/resource loading | `bbb-pack` asset-index fallback | `equivalent` | full asset audit 与 unihex real load |
| 共同位置序列 | server teleport → player-position packet | replay generator + play apply | `equivalent` | camera-log deterministic hash |
| 认证服务器 | `authenticateServer` online path | 离线 server 配置 | `not-needed` | online mode explicitly disabled |

更多采样和 invalid-run 规则见[等负载比较](../mechanisms/performance/equal-load-benchmark.md)。
