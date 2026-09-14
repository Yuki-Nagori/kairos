// 为 cargo-llvm-cov 定位与 rustc 同版本的 llvm-cov / llvm-profdata。
// rustup 管理的 rustc 自带查找路径，无需本脚本干预；Homebrew / 发行版 rustc
// 不附带 llvm-tools（覆盖率工具链），此时从 rustup 工具链目录或系统 LLVM
// 解析，经 LLVM_COV / LLVM_PROFDATA 传给 cargo-llvm-cov。
// 本脚本只做工具解析与参数透传，cargo 参数（含覆盖门槛）由 package.json 给出。
//
// 门槛（--fail-under-lines 等）只给汇总数字，「哪几行没覆盖」得另找——llvm-cov 的行口径
// 与源码直觉并不一致：闭包体是独立落点（外层行每次都跑到、闭包体没执行也算未覆盖），
// 同一段泛型被多份单态实例实现时同一行还会被登记两次（其中提前返回的那份计数为 0）。
// 因此门槛没过时这里补一次 JSON 导出，按文件列出两类落点，省去逐层反查。
import { execSync, spawnSync } from "node:child_process";
import { existsSync, readFileSync, rmSync } from "node:fs";
import { homedir, tmpdir } from "node:os";
import { delimiter, join, relative } from "node:path";

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
const args = process.argv.slice(2);

/** 报告形态类参数：落点分析要自己接管形态，避免与 --summary-only / --html 冲突。 */
const REPORT_FLAGS = ["--summary-only", "--text", "--html", "--lcov", "--cobertura", "--json"];

/** 带值的报告参数：`--output-path X` 与 `--output-path=X` 两种写法都要摘掉。 */
const VALUED_REPORT_FLAGS = [
  "--output-path",
  "--output-dir",
  "--fail-under-lines",
  "--fail-under-regions",
  "--fail-under-functions",
];

/** 摘掉报告形态与门槛参数，只留「跑哪些测试」那部分（落点分析复用同一次构建）。 */
const stripReportFlags = (passthrough: string[]): string[] => {
  const kept: string[] = [];
  let eatNext = false;
  for (const arg of passthrough) {
    if (eatNext) {
      eatNext = false;
      continue;
    }
    if (REPORT_FLAGS.includes(arg)) continue;
    const valued = VALUED_REPORT_FLAGS.find((flag) => arg === flag || arg.startsWith(`${flag}=`));
    if (valued) {
      eatNext = arg === valued;
      continue;
    }
    kept.push(arg);
  }
  return kept;
};

/** llvm-cov JSON 导出的最小字段集：region 的文件号是所在函数 `filenames` 表的下标。 */
type Region = readonly [number, number, number, number, number, number, number, number];

interface CoverageFunction {
  filenames: string[];
  regions: Region[];
}

interface CoverageExport {
  data: Array<{
    files: Array<{ filename: string; summary: { lines: { count: number; covered: number } } }>;
    functions: CoverageFunction[];
  }>;
}

/** 一个文件里的两类落点：整行区域全 0（确实没执行）、行内另有 0 计数区域（独立落点没执行）。 */
const linesByCount = (
  filename: string,
  functions: CoverageFunction[],
): { never: number[]; partial: number[] } => {
  const counts = new Map<number, number[]>();
  for (const fn of functions) {
    for (const region of fn.regions) {
      const [lineStart, , lineEnd, , count, fileId] = region;
      if (fn.filenames[fileId] !== filename) continue;
      for (let line = lineStart; line <= lineEnd; line += 1) {
        const seen = counts.get(line);
        if (seen) seen.push(count);
        else counts.set(line, [count]);
      }
    }
  }
  const never: number[] = [];
  const partial: number[] = [];
  for (const [line, regionCounts] of counts) {
    if (Math.max(...regionCounts) === 0) never.push(line);
    else if (Math.min(...regionCounts) === 0) partial.push(line);
  }
  const ascending = (left: number, right: number): number => left - right;
  return { never: never.sort(ascending), partial: partial.sort(ascending) };
};

/** 每类落点最多列这么多行，避免一个文件把终端刷满。 */
const MAX_LISTED_LINES = 12;

const printLines = (heading: string, lines: number[], source: string[]): void => {
  if (lines.length === 0) return;
  console.error(`    ${heading}（${lines.length} 行）：`);
  for (const line of lines.slice(0, MAX_LISTED_LINES)) {
    console.error(`      L${line}: ${(source[line - 1] ?? "").trim()}`);
  }
  if (lines.length > MAX_LISTED_LINES) {
    console.error(`      …另有 ${lines.length - MAX_LISTED_LINES} 行`);
  }
};

/** 门槛没过时补一次 JSON 导出：汇总只有数字，落点得按行列出来才修得动。 */
const reportUncoveredLines = (passthrough: string[]): void => {
  const jsonPath = join(tmpdir(), `kairos-coverage-${process.pid}.json`);
  // `--` 之后的参数归测试进程（`--nocapture` 之类），报告参数只能加在它前面。
  const separator = passthrough.indexOf("--");
  const cargoArgs = separator < 0 ? passthrough : passthrough.slice(0, separator);
  const testArgs = separator < 0 ? [] : passthrough.slice(separator);
  const diagnostic = [
    ...stripReportFlags(cargoArgs),
    "--json",
    `--output-path=${jsonPath}`,
    ...testArgs,
  ];
  console.error(`\ncoverage:rust 门槛未达标，导出逐行数据：cargo llvm-cov ${diagnostic.join(" ")}`);
  // 这一次只是取数据：输出收起来，只在没拿到 JSON 时回显（构建 / 测试失败的原因在那里）。
  const second = spawnSync("cargo", ["llvm-cov", ...diagnostic], {
    stdio: ["ignore", "pipe", "pipe"],
    env: { ...process.env, ...env },
    shell: process.platform === "win32",
  });
  if (!existsSync(jsonPath)) {
    console.error("  逐行数据没生成（构建 / 测试本身失败？），跳过落点分析。");
    console.error(second.stderr?.toString().trim() ?? "");
    return;
  }
  try {
    const exported = JSON.parse(readFileSync(jsonPath, "utf8")) as CoverageExport;
    const { files, functions } = exported.data[0];
    const failing = files.filter((file) => file.summary.lines.covered < file.summary.lines.count);
    if (failing.length === 0) {
      console.error("  没有行覆盖低于 100% 的文件：门槛失败可能来自函数 / 区域覆盖或测试失败。");
      return;
    }
    for (const file of failing) {
      const missed = file.summary.lines.count - file.summary.lines.covered;
      const source = readFileSync(file.filename, "utf8").split("\n");
      const { never, partial } = linesByCount(file.filename, functions);
      console.error(`\n  ${relative(process.cwd(), file.filename)}：未覆盖 ${missed} 行`);
      printLines("整行未执行", never, source);
      printLines("行内有 0 计数区域（闭包体没执行 / 多份单态实例的重复记账）", partial, source);
    }
    console.error("\n  两类落点的成因与应对见 ai-docs/ARCHITECTURE.md §6「假未覆盖」；逐行 HTML：");
    console.error("  cargo llvm-cov -p kairos-core --lib --html --output-dir target/coverage-html");
  } catch (error) {
    // 分析失败不影响门槛结论：原始退出码照旧，只是这次没给出落点。
    console.error(`  落点分析失败：${error instanceof Error ? error.message : String(error)}`);
  } finally {
    rmSync(jsonPath, { force: true });
  }
};

const result = spawnSync("cargo", ["llvm-cov", ...args], {
  stdio: "inherit",
  env: { ...process.env, ...env },
  // Windows 的 spawn 不走 PATH 解析可执行文件，需要经 shell 调起 cargo
  shell: process.platform === "win32",
});

// 门槛没过（cargo-llvm-cov 退出码 1）才有落点可报；构建 / 测试失败照旧原样退出。
if (result.status === 1 && args.some((arg) => arg.startsWith("--fail-under"))) {
  reportUncoveredLines(args);
}
process.exit(result.status ?? 1);
