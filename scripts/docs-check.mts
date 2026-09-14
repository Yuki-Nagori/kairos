// 文档体检：内部链接必须指向存在的文件；除索引外每篇文档都应被引用。
//
// 断链会让读者（与人以外的读者——编码代理）照着路径找不到东西；孤儿文档则是
// 「写了但没人能找到」，两条都是纯机械判断，适合固化成门禁而不是靠人肉自查。
//
// 用法：bun run docs:check
// 退出码：断链 → 1；孤儿 → 只报告不失败（评审/预研类文档允许在索引里成组列出）。
import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";

const ROOT = "ai-docs";
/** 索引文件本身不要求被别处引用。 */
const INDEX_FILES = new Set(["README.md"]);

/** 递归收集 markdown 文档。 */
function collect(dir: string): string[] {
  const out: string[] = [];
  for (const entry of readdirSync(dir)) {
    const path = join(dir, entry);
    if (statSync(path).isDirectory()) {
      out.push(...collect(path));
    } else if (entry.endsWith(".md")) {
      out.push(path);
    }
  }
  return out;
}

/** 提取 markdown 里的相对链接目标（跳过 http/mailto 与纯锚点）。 */
function links(text: string): string[] {
  const targets: string[] = [];
  for (const match of text.matchAll(/\[[^\]]*\]\(([^)\s]+)\)/g)) {
    const target = match[1].split("#")[0];
    if (target.length === 0 || /^(https?:|mailto:)/.test(target)) {
      continue;
    }
    targets.push(target);
  }
  return targets;
}

const docs = collect(ROOT).sort();
const broken: string[] = [];
const referenced = new Set<string>();

for (const doc of docs) {
  const text = readFileSync(doc, "utf8");
  for (const target of links(text)) {
    const resolved = resolve(dirname(doc), target);
    if (!statSync(resolved, { throwIfNoEntry: false })) {
      broken.push(`${relative(ROOT, doc)} → ${target}`);
      continue;
    }
    referenced.add(resolved);
  }
}

// 任务档案由 tasks/README.md 表格逐行引用；评审 / 预研 / 决策允许在索引里成组
// 提及（列出文件名即可），因此这里用「文件名是否出现在任何文档正文里」判断。
const bodies = docs.map((doc) => readFileSync(doc, "utf8")).join("\n");
const orphans = docs
  .filter((doc) => !INDEX_FILES.has(doc.split("/").pop() ?? ""))
  .filter((doc) => !referenced.has(resolve(doc)) && !bodies.includes(doc.split("/").pop() ?? ""))
  .map((doc) => relative(ROOT, doc));

console.log(`文档体检：${docs.length} 篇`);
if (broken.length > 0) {
  console.error(`断链 ${broken.length} 处：`);
  for (const item of broken) {
    console.error(`  - ${item}`);
  }
}
if (orphans.length > 0) {
  console.warn(`未被引用的文档 ${orphans.length} 篇（建议补进索引）：`);
  for (const item of orphans) {
    console.warn(`  - ${item}`);
  }
}
if (broken.length === 0 && orphans.length === 0) {
  console.log("断链 0，孤儿 0。");
}
process.exit(broken.length > 0 ? 1 : 0);
