/**
 * 求解环境部署状态比对（「更新未部署」提醒）：本机下载的 bundle 版本
 * 与 VM 内已部署版本的纯函数比对。部署概念仅存在于 multipass 通道。
 */

/**
 * 是否存在「已下载新环境但 VM 内未部署」的错位：
 * - 下载侧无版本标签（静态直链组件）→ 无法比对，不提示；
 * - VM 侧未知（未部署过 / VM 未启动 / 非 multipass）→ 有下载即提示；
 * - 两侧标签相等 → 已是最新；不等 → 提示。
 */
export function isPendingDeploy(downloadedTag: string | null, deployedTag: string | null): boolean {
  if (downloadedTag === null) {
    return false;
  }
  return downloadedTag !== deployedTag;
}
