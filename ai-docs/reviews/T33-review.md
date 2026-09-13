# T33 评审记录（T21 清单 · 随任务执行，部分实现）

评审对象：`src-tauri/tauri.conf.json` 的生产 CSP、capabilities、`ai-docs/release.md` 的
签名 / updater 约定。

| #   | 清单项         | 结论                                                                                                                                                                                                         |
| --- | -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | 架构一致性     | 通过。仅改安全配置，不动分层与契约                                                                                                                                                                           |
| 2   | 性能预算对照   | 不适用（配置项）                                                                                                                                                                                             |
| 3   | 代码与注释质量 | 通过。CSP 新增项在 release.md 里逐条写明「为什么安全」（对应能力全仓未使用）                                                                                                                                 |
| 4   | 文档同步       | 通过。release.md 的 CSP 段从「一条结论」扩为「已生效 / 待办」两段，写明待办项为什么必须真机验证                                                                                                              |
| 5   | 测试覆盖缺口   | 不适用（配置）；构建校验：`bun run build` + `cargo check` 通过                                                                                                                                               |
| 6   | 依赖健康       | 无新增依赖（updater 插件未接入——见下）                                                                                                                                                                       |
| 7   | 安全           | **本轮加固**：新增 `script-src 'self'`（显式）、`object-src 'none'`、`base-uri 'self'`、`frame-ancestors 'none'`、`form-action 'none'`；capabilities 未动（仍只有 main 窗口 + dialog/decoration + 窗口控制） |
| 8   | 上轮遗留       | T18/release.md 的「`style-src 'unsafe-inline'` 待主题文件化」在本轮明确了改造路径与验证前提                                                                                                                  |

## 本轮做了什么

- **CSP 加固（零行为变化的部分）**：`object-src 'none'`（应用无 `<object>`/`<embed>`）、
  `base-uri 'self'`（无 `<base>`）、`frame-ancestors 'none'`（桌面应用从不被嵌入）、
  `form-action 'none'`（无 `<form>` 提交路径）、`script-src 'self'` 显式化。
  这些限制对应的能力在 `src-web` 与 `index.html` 里全部为 0 命中（已 grep 核对），
  因此不会破坏现有功能。

## 未做的部分（明确记录，需外部条件）

- **`style-src 'unsafe-inline'` 移除**：主题在运行时把 `theme/*.css` 文本注入
  `<style>` 元素（`useTheme` 的 `style.textContent = css`），移除后主题切换与
  动态 `:style` 绑定（色标渐变等）会失效。改造路径（二选一）：
  ① 构建期把主题拆成独立 CSS 文件、用 `<link rel="stylesheet">` 切换；
  ② 保留注入但对 `<style>` 打 nonce，并在 Tauri 侧配置该 nonce。
  两者都是**行为性改动**，验收必须真机回归（主题切换 + 色彩图例 + IPC + 下载），
  当前无桌面控制权限 → 归 GUI 走查批次。
- **updater 插件接入**：`tauri-plugin-updater` 需要签名密钥对（`tauri signer generate`）
  与 `plugins.updater.pubkey`，release.yml 还要注入 Secrets 并产出 `.sig` / `latest.json`。
  密钥由发布者持有，无法代生成；接入清单（端点 JSON 格式、签名清单生成脚本、回滚约定）
  已写在 release.md，等证书 / 密钥就绪即可执行。
- **三平台签名（notarytool / signtool / Linux 包签名）**：同上，依赖证书。
