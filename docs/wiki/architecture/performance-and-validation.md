# 性能与验证边界

## 正确性先于比较

官方 Vanilla 26.1 渲染机制与可见 workload 等价是性能比较的前置门禁。可见几何、
材质、光照、透明面、实体、粒子或 resident/visible 数量不一致时，对应 run 作废；
减少 draw workload 不能称为 renderer 优化。

验证顺序是：确定性输入 → 机制/几何断言 → 同 tick 锚点与 material-specific ROI →
共同 draw evidence → 外部 frame/CPU 采样。OpenGL 与 Vulkan 允许细微 raster 差异，
所以截图 hash 不要求相等；但全图平均误差也不能掩盖局部缺块、错误面或亮度级差。

## 指标口径

- MangoHud present 边界记录 end-to-end application-frame interval，不是纯 CPU 或 GPU
  execution duration。
- `perf stat` 记录客户端进程总 task-clock、cycles、instructions 等，只能按实际 frame
  数归一化为均值，不能产生 CPU frame percentile。
- GPU load/clock/temperature 是采样 telemetry，不是 per-frame GPU duration。
- API-neutral per-client GPU execution duration 只有在能跨 radeonsi/RADV 按客户端
  context 关联 submit、queue、fence 且排除 compositor 时才可发布；目前其外部验证边界
  仍链接[缺口 ledger](../../unsupported-features.md)。

完整合同见[官方客户端等负载比较](../mechanisms/performance/equal-load-benchmark.md)，构建
基线和 repo-native owner 证据见[构建与缓存](../development/build-and-cache.md)。
