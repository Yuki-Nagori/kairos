// 注释规范门禁：按 ai-docs/comment-style.md §3/§5 禁止代码里出现内部规划引用
// （任务编号）、对标出处、开发过程叙事与评审文档路径。注释过期与规划信息漂进
// 代码是评审反复发现的问题形态，这里用脚本固化，避免靠人肉自查。
//
// 只扫代码文件（Rust / TS / Vue），文档（ai-docs）是规划信息的正确去处，不在此列。
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

/** 违例模式 → 说明（与 comment-style.md §3/§5 对应）。 */
const RULES = [
  { pattern: /\bT\d{2}\b/, label: "任务编号引用（Txx 属 ai-docs/tasks）" },
  { pattern: /Moldflow|对标|参考自|基于\s*Moldflow/, label: "对标出处说明" },
  { pattern: /真机/, label: "开发过程叙事（真机…）" },
  { pattern: /ai-docs\/reviews|timeline\.md/, label: "评审 / 时间线文档路径" },
  { pattern: /20\d{2}-\d{2}-\d{2}/, label: "日期戳（背景信息用一句结论表达）" },
];

const ROOTS = ["src-crates", "src-tauri", "src-web"];
const EXTENSIONS = [".rs", ".ts", ".vue"];

function collect(dir, files = []) {
  for (const name of readdirSync(dir)) {
    if (name === "node_modules" || name === "target" || name === "dist") continue;
    const path = join(dir, name);
    if (statSync(path).isDirectory()) {
      collect(path, files);
    } else if (EXTENSIONS.some((ext) => name.endsWith(ext))) {
      files.push(path);
    }
  }
  return files;
}

const violations = [];
for (const root of ROOTS) {
  for (const file of collect(root)) {
    readFileSync(file, "utf8")
      .split("\n")
      .forEach((line, index) => {
        for (const rule of RULES) {
          if (rule.pattern.test(line)) {
            violations.push(`${file}:${index + 1}  ${rule.label}\n    ${line.trim()}`);
          }
        }
      });
  }
}

if (violations.length > 0) {
  console.error(`注释规范检查未通过（${violations.length} 处）：\n`);
  for (const violation of violations) console.error(`  ${violation}\n`);
  console.error("规范见 ai-docs/comment-style.md：规划信息写进 ai-docs，代码注释只留结论。");
  process.exit(1);
}
console.log("注释规范检查通过（无任务编号 / 对标 / 过程叙事 / 日期戳）。");
