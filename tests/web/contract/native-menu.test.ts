/**
 * 菜单动作 id 契约：Rust 原生菜单引用的每个动作 id 必须存在于前端
 * menu-actions 动作表（TS ↔ Rust 无跨语言类型约束，靠本契约测试拦截漂移）。
 * 通过解析双方源码而非执行，保持与运行环境解耦。
 */
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

// vitest 以仓库根为 cwd 运行
const ROOT = process.cwd();

/** 从 lib.rs 提取原生菜单引用的动作 id（action("<id>", ...) 调用点）。 */
function rustMenuIds(): string[] {
  const source = readFileSync(join(ROOT, "src-tauri/src/lib.rs"), "utf8");
  return [...source.matchAll(/action\(\s*"([^"]+)"/g)].map((match) => match[1]!);
}

/** 从 menu-actions.ts 提取前端动作表的所有 id（"x.y": 键）。 */
function frontendActionIds(): string[] {
  const source = readFileSync(join(ROOT, "src-web/menu-actions.ts"), "utf8");
  return [...source.matchAll(/"([a-zA-Z]+\.[a-zA-Z]+)":/g)].map((match) => match[1]!);
}

describe("原生菜单 ↔ 前端动作表 契约", () => {
  it("Rust 菜单引用的动作 id 都在前端动作表中", () => {
    const frontend = new Set(frontendActionIds());
    const missing = rustMenuIds().filter((id) => !frontend.has(id));
    expect(missing).toEqual([]);
  });

  it("Rust 菜单无重复动作 id", () => {
    const ids = rustMenuIds();
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("Rust 菜单覆盖核心动作集（防误删回漂；app.about 为纯前端动作不含在内）", () => {
    const required = [
      "file.new",
      "file.open",
      "file.save",
      "file.saveAs",
      "view.theme",
      "analysis.checkNetwork",
      "results.exportCsv",
      "tools.refreshDeps",
      "tools.vmPanel",
      "tools.vmStart",
      "tools.vmShell",
      "tools.vmStop",
      "report.open",
    ];
    const ids = new Set(rustMenuIds());
    const missing = required.filter((id) => !ids.has(id));
    expect(missing).toEqual([]);
  });
});
