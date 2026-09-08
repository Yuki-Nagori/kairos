import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { appStore, initialAppState } from "../../../src-web/state";
import type { Project } from "../../../src-web/types";
import { createProjectBar } from "../../../src-web/components/project-bar";

const project: Project = {
  schemaVersion: 1,
  id: "p-1",
  name: "演示项目",
  createdMs: 1,
  updatedMs: 1,
  studies: [
    {
      id: "s-1",
      name: "填充分析",
      createdMs: 2,
      runnerElements: [],
      coolingChannels: [],
      process: null,
      materialId: null,
    },
  ],
};

describe("createProjectBar", () => {
  beforeEach(() => {
    appStore.set(initialAppState);
  });

  afterEach(() => {
    appStore.set(initialAppState);
  });

  it("shows placeholder and disables study editing without a project", () => {
    const bar = createProjectBar();
    expect(bar.textContent).toContain("未打开项目");
    const studyInput = [...bar.querySelectorAll("input")].find((input) =>
      input.placeholder.includes("新研究"),
    );
    expect(studyInput?.disabled).toBe(true);
  });

  it("renders project name and study chips, and removing works", () => {
    appStore.set({ project });
    const bar = createProjectBar();
    expect(bar.textContent).toContain("演示项目");
    expect(bar.textContent).toContain("填充分析");

    const remove = bar.querySelector('span[title^="删除研究"]');
    expect(remove).not.toBeNull();
    (remove as HTMLElement).click();
    expect(appStore.get().project?.studies).toHaveLength(0);
  });
});
