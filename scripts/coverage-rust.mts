// 为 cargo-llvm-cov 定位与 rustc 同版本的 llvm-cov / llvm-profdata。
// rustup 管理的 rustc 自带查找路径，无需本脚本干预；Homebrew / 发行版 rustc
// 不附带 llvm-tools（覆盖率工具链），此时从 rustup 工具链目录或系统 LLVM
// 解析，经 LLVM_COV / LLVM_PROFDATA 传给 cargo-llvm-cov。
// 本脚本只做工具解析与参数透传，cargo 参数（含覆盖门槛）由 package.json 给出。
import { execSync, spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { homedir } from "node:os";
import { delimiter, join } from "node:path";

const query = (command: string): string | null => {
  try {
    return execSync(command, { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] }).trim();
  } catch {
    return null;
  }
};

const hasTool = (dir: string, name: string): boolean =>
  existsSync(join(dir, name)) || existsSync(join(dir, `${name}.exe`));

const dirWithBothTools = (dir: string): string | null =>
  dir && hasTool(dir, "llvm-cov") && hasTool(dir, "llvm-profdata") ? dir : null;

const toolDirs = (): string[] => {
  const dirs: string[] = [];
  const sysroot = query("rustc --print sysroot");
  const info = query("rustc -vV");
  const host = info?.match(/^host:\s*(\S+)/m)?.[1];
  const release = info?.match(/^release:\s*(\S+)/m)?.[1];

  // 1. 活动 rustc 的 sysroot（纯 rustup 环境此处即 llvm-tools-preview 安装位置）
  if (sysroot && host) dirs.push(join(sysroot, "lib", "rustlib", host, "bin"));

  // 2. RUSTUP_HOME 中与活动 rustc 同版本的工具链（混合环境：PATH 上的 rustc 非 rustup 管理）
  const rustupHome = process.env.RUSTUP_HOME ?? join(homedir(), ".rustup");
  if (release && host) {
    dirs.push(join(rustupHome, "toolchains", `${release}-${host}`, "lib", "rustlib", host, "bin"));
  }

  // 3. 系统 LLVM（Homebrew 版本化在前，避免主版本漂移）
  if (process.platform === "darwin") {
    for (const formula of ["llvm@22", "llvm"]) {
      const prefix = query(`brew --prefix ${formula}`);
      if (prefix) dirs.push(join(prefix, "bin"));
    }
  }
  return dirs;
};

const resolveTools = (): Record<string, string> | null => {
  if (process.env.LLVM_COV && process.env.LLVM_PROFDATA) return {};
  const onPath = (name: string): boolean =>
    (process.env.PATH ?? "").split(delimiter).some((dir) => dir !== "" && hasTool(dir, name));
  if (onPath("llvm-cov") && onPath("llvm-profdata")) return {};
  const dir = toolDirs().find((candidate) => dirWithBothTools(candidate));
  if (!dir) {
    console.error("coverage:rust 未能定位 llvm-cov / llvm-profdata，交给 cargo-llvm-cov 原生报错");
    return {};
  }
  console.error(`coverage:rust 使用 LLVM 工具链目录：${dir}`);
  const extension = process.platform === "win32" ? ".exe" : "";
  return {
    LLVM_COV: join(dir, `llvm-cov${extension}`),
    LLVM_PROFDATA: join(dir, `llvm-profdata${extension}`),
  };
};

const env = resolveTools();
const result = spawnSync("cargo", ["llvm-cov", ...process.argv.slice(2)], {
  stdio: "inherit",
  env: { ...process.env, ...env },
  // Windows 的 spawn 不走 PATH 解析可执行文件，需要经 shell 调起 cargo
  shell: process.platform === "win32",
});
process.exit(result.status ?? 1);
