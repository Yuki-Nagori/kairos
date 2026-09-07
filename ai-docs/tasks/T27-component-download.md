# T27 · 应用内组件下载

- 阶段：M4 后处理与体验
- 依赖：T09（运行时集成）
- 优先级：P1

## 目标

在应用内提供外部运行时组件（Gmsh、OpenFOAM 7、openInjMoldSim）的下载入口：按平台选择官方二进制地址，流式下载到受管目录，进度百分比实时反馈。

## 范围

- 下载白名单（gmsh.info / openfoam.org / github.com/OpenFOAM / github.com/krebeljk）；
- 流式写盘 + Channel 进度回传；
- 下载目录路径展示与打开。

## 非目标

- 下载完成后的自动安装/解压（后续迭代）；
- 下载校验和（后续迭代）。

## 交付物

- `commands/downloads.rs`：download_file / get_downloads_dir / open_downloads_dir 命令；
- 依赖面板中的下载按钮与进度条。

## 验收标准

- 下载进度实时更新；文件落在受管目录；白名单外 URL 被拒绝。
