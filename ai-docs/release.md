# 发布流程（T18）

## 版本与标签

1. 更新 `src-tauri/tauri.conf.json` 与根 `Cargo.toml`（workspace）版本号；
2. 提交并打标签：`git tag -a v0.x.0 -m "..."` 并推送标签。

## CI 流水线

推送 `v*` 标签后，GitHub Actions 自动：

1. 运行全量门禁（verify 分解：typecheck / lint / format / test / knip）；
2. macOS 与 Windows 双平台 `bun run tauri build`；
3. 收集 `target/release/bundle` 下的 .dmg / .app / .msi / .exe 并上传至 GitHub Release 草稿。

## 签名（证书就绪后启用）

- macOS：`codesign --deep --force --sign "Developer ID Application: ..."` + `notarytool` 公证；
- Windows：`signtool sign /fd SHA256 /tr <TSA> ...`；
- 证书与密钥不入库（GitHub Secrets 管理）。

## 自动更新（预留）

Tauri v2 updater 插件需要：

1. 更新端点（静态 JSON 或服务端）；
2. 生成的签名密钥对（公钥入配置，私钥保密）；
3. 每次发布附带签名清单。

密钥与端点就绪后启用 `tauri.conf.json` 的 `plugins.updater` 段并安装 `tauri-plugin-updater`。
