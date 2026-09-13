# T73 · 下载面板显示 release 版本号（同名资产歧义）

- 阶段：D0（平台底座 · 依赖管理）
- 依赖：T27（应用内组件下载）、T54（部署版本标记）
- 优先级：**P3**
- 状态：**已解决（上游资产改名，Kairos 侧无需改动）**

> 2026-09-13 上游已把资产名改为 `moldingFoam-<version>-<arch>.tar.xz`：
> 文件名自带版本号，面板上「已下载 `moldingFoam-v0.2.4-linuxArm64.tar.xz`」即可读出
> 版本，同名歧义消失。Kairos 侧只做了配套加固（资产架构匹配放宽到 aarch64 /
> arm64 / x86_64 / amd64 等写法，并排除同一 release 里的 macOS / Windows 资产）。

## 现象（2026-09-13 复测遇到）

moldingFoam 的 v0.2.2 与 v0.2.3 两个 release 的资产**文件名完全相同**
（`moldingFoam-openfoam14-linuxArm64GccDPInt32Opt-20260913.tar.xz`），而依赖面板
「已下载」行只显示文件名（且被截断）：

```
已下载 moldingFoam-openfoam14-linuxArm64GccDPInt32Opt-20260913.…
```

于是「本地这份是哪个版本」在界面上不可知——本轮据此误判过「已经下到最新」，
实际磁盘上是 v0.2.2（`manifest.json` 的 `releaseTag` 才是权威）。

## 范围

1. 「已下载」行主显 `releaseTag`（无 tag 的组件回退文件名），如
   `已下载 v0.2.3 · 113.8 MB · 已解压`；文件名降为次要信息（title 或副行）；
2. 「有更新未部署」提示已按 tag 比对（T54）✓ 保持；
3. 组件测试：有 tag / 无 tag 两种渲染。

## 非目标

- 资产改名（属上游发布约定）；
- 下载校验（哈希比对）——可另开任务。

## 验收标准

- 面板上能直接读出本地已下载的 release 版本；
- `bun run verify` 全绿。
