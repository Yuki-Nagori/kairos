# T33 · 发布加固：自动更新与 CSP 收窄

- 阶段：下一阶段（分发质量）
- 依赖：T18（发布流水线，已完成）
- 优先级：P3（受代码签名证书可用性约束）

## 目标

兑现 T18/release.md 的预留项：Tauri updater 自动更新插件接入、生产 CSP 收窄、（证书就绪后）三平台签名。

## 范围

- `tauri-plugin-updater` 接入：更新端点（静态 JSON 起步）、签名密钥对生成与保管约定（公钥入配置，私钥进 Secrets）；
- release.yml 构建 updater 签名清单（`tauri build` 的 updater 产物）；
- 生产 CSP 从现有白名单收窄：`style-src 'unsafe-inline'` 评估移除（主题注入改 nonce 或文件化）、`connect-src` 收敛到实际端点 + updater 端点；
- 证书就绪后追加：macOS 公证（notarytool）、Windows signtool、Linux 包签名步骤。

## 非目标

- 更新服务端开发（静态 JSON / 对象存储即可）；
- 差量更新。

## 交付物

- updater 插件集成 + 端点配置说明（release.md 扩充）；
- 收窄后的 CSP 与回归记录；
- 签名步骤（证书就绪部分）。

## 验收标准

- 模拟更新流程：旧版本检测到新版本并完成更新（本地静态服务验证）；
- 收窄后 CSP 下全部功能（主题注入、IPC、下载、updater）回归正常；
- `cargo audit` 无高危未处理项。
