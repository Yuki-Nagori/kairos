# 发布流程（T18）

## 版本与标签

版本号**只有一处事实源**：根 `Cargo.toml` 的 `[workspace.package] version`。派生关系：

- 两个成员 crate（`src-tauri` / `kairos-core`）用 `version.workspace = true` 继承；
- `tauri.conf.json` 缺省 `version`，tauri-codegen 回退 `CARGO_PKG_VERSION`；
- `package.json` 不存版本（前端私有包，运行时版本来自 Rust `system_info`）。

发布动作：

1. 到 GitHub Actions 页手动运行 Release 工作流，填入发布标签（如 `v0.x.0`）——
   工作流自动把该版本写入 `Cargo.toml`（Cargo.lock 由构建自动改写）；
2. 标签会在本次构建的提交上自动创建，无需本地打标签推送。

## CI 流水线

手动触发 Release 工作流后，GitHub Actions 自动：

1. 三平台构建：macOS（arm64）、Windows（x64）、Ubuntu（x64）各自 `bun run tauri build`；
2. 收集各平台 bundle 下的 .dmg / .app / .msi / .exe / .deb / .rpm / .AppImage；
3. 汇总为 Release **草稿**，挂在工作流输入的标签上——人工核对后手动发布。

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

## T33 · 自动更新与签名（证书就绪后启用）

- 端点：静态 JSON（`https://releases.kairos.example/latest.json`，占位域名，
  正式发布时替换为对象存储直链）；
- 签名密钥：`bunx tauri signer generate -w ~/.tauri/kairos.key`——公钥写入
  `tauri.conf.json` 的 `plugins.updater.pubkey`，私钥放 GitHub Secrets
  （`TAURI_SIGNING_PRIVATE_KEY` / `_PASSWORD`）；
- release.yml 在 `tauri build` 前注入上述 Secrets 即可产出
  `.sig` 更新签名清单；`latest.json` 按官方 updater 格式手写生成脚本（后续）；
- CSP 收窄结论：`img-src` 已去掉 `blob:`（无运行时 blob 图片）；
  `style-src 'unsafe-inline'` 暂保留——主题切换依赖运行时 `<style>` 注入，
  移除需 nonce 化改造（记录为接受项，待主题文件化时重评）。
