import { beforeEach, describe, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import StageRibbon from "../../../src-web/components/stage-ribbon/StageRibbon.vue";
import {
  STAGE_HINTS,
  useStageRibbon,
} from "../../../src-web/components/stage-ribbon/useStageRibbon";
import { useAppStore } from "../../../src-web/stores/app";
import { useGeometryStore } from "../../../src-web/stores/geometry";
import { useJobsStore } from "../../../src-web/stores/jobs";
import { useResultsStore } from "../../../src-web/stores/results";
import type { ScalarField } from "../../../src-web/types";

vi.mock("../../../src-web/menu-actions", () => ({
  runMenuAction: vi.fn(),
}));

import { runMenuAction } from "../../../src-web/menu-actions";

/** 把 app store 切到指定阶段（工具条分组随阶段变化）。 */
function gotoStage(stage: string): void {
  useAppStore().stage = stage as never;
}

describe("useRibbon 分组结构", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    vi.mocked(runMenuAction).mockClear();
  });

  it("主页：文件动作 + 校验/探测两组", () => {
    gotoStage("home");
    const { groups } = useStageRibbon();
    expect(groups.value.map((group) => group.buttons.length)).toEqual([3, 2]);
    groups.value.flatMap((group) => group.buttons).forEach((button) => button.run());
    expect(runMenuAction).toHaveBeenCalledWith("file.new");
    expect(runMenuAction).toHaveBeenCalledWith("file.open");
    expect(runMenuAction).toHaveBeenCalledWith("file.save");
    expect(runMenuAction).toHaveBeenCalledWith("analysis.checkNetwork");
    expect(runMenuAction).toHaveBeenCalledWith("tools.refreshDeps");
  });

  it("几何：导入动作调几何 store 且忙碌时禁用", async () => {
    gotoStage("geometry");
    const geometry = useGeometryStore();
    const importSpy = vi.spyOn(geometry, "importGeometry").mockReturnValue(Promise.resolve());
    const sampleSpy = vi
      .spyOn(geometry, "importSampleGeometry")
      .mockReturnValue(Promise.resolve(undefined));
    const { groups } = useStageRibbon();
    const buttons = groups.value[0]!.buttons;
    expect(buttons.map((button) => button.label)).toEqual(["导入 STL", "导入样例"]);
    expect(buttons.every((button) => button.disabled?.() === false)).toBe(true);

    buttons[0]!.run();
    buttons[1]!.run();
    await Promise.all([importSpy.mock.results[0]!.value, sampleSpy.mock.results[0]!.value]);
    expect(importSpy).toHaveBeenCalledOnce();
    expect(sampleSpy).toHaveBeenCalledOnce();

    useAppStore().busy = "忙碌中";
    expect(buttons.every((button) => button.disabled?.() === true)).toBe(true);
  });

  it("工艺：校验模具网络", () => {
    gotoStage("process");
    const { groups } = useStageRibbon();
    expect(groups.value[0]!.buttons[0]!.label).toBe("校验模具网络");
    groups.value[0]!.buttons[0]!.run();
    expect(runMenuAction).toHaveBeenCalledWith("analysis.checkNetwork");
  });

  it("求解：刷新作业调 store，探测依赖走动作，忙碌时禁用", async () => {
    gotoStage("solve");
    const jobs = useJobsStore();
    const refreshSpy = vi.spyOn(jobs, "refreshJobs").mockReturnValue(Promise.resolve());
    const { groups } = useStageRibbon();
    const [refresh, deps] = groups.value[0]!.buttons.concat(groups.value[1]!.buttons);
    expect(refresh!.label).toBe("刷新作业");
    refresh!.run();
    await refreshSpy.mock.results[0]!.value;
    expect(refreshSpy).toHaveBeenCalledOnce();
    expect(runMenuAction).not.toHaveBeenCalled();

    expect(deps!.label).toBe("探测依赖");
    deps!.run();
    expect(runMenuAction).toHaveBeenCalledWith("tools.refreshDeps");

    useAppStore().busy = "x";
    expect(refresh!.disabled?.()).toBe(true);
  });

  it("结果：已加载场时可导出，无场时导出禁用；重扫调 rescanCatalog", async () => {
    gotoStage("results");
    const results = useResultsStore();
    const rescanSpy = vi.spyOn(results, "rescanCatalog").mockReturnValue(Promise.resolve());
    const { groups } = useStageRibbon();
    const [exportBtn, rescanBtn] = groups.value[0]!.buttons;
    expect(exportBtn!.disabled?.()).toBe(true);

    results.loadedField = {
      field: "T",
      timeDir: "0.1",
      timeS: 0.1,
      isMagnitude: false,
      complete: true,
      values: [1],
    } satisfies ScalarField;
    expect(exportBtn!.disabled?.()).toBe(false);
    exportBtn!.run();
    expect(runMenuAction).toHaveBeenCalledWith("results.exportCsv");

    rescanBtn!.run();
    await rescanSpy.mock.results[0]!.value;
    expect(rescanSpy).toHaveBeenCalledOnce();
  });

  it("网格 / 报告阶段无工具分组，渲染阶段引导文案", () => {
    for (const stage of ["mesh", "report"] as const) {
      gotoStage(stage);
      const { groups, hint } = useStageRibbon();
      expect(groups.value).toEqual([]);
      expect(hint.value).toBe(STAGE_HINTS[stage]);
    }
  });
});

describe("StageRibbon 组件", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    vi.mocked(runMenuAction).mockClear();
  });

  it("渲染图标 + 标签按钮，点击执行动作", async () => {
    const wrapper = mount(StageRibbon, { global: { plugins: [pinia] } });
    const buttons = wrapper.findAll("button");
    expect(buttons.length).toBe(5);
    expect(buttons[0]!.text()).toContain("新建项目");
    expect(buttons[0]!.text()).toContain("＋");

    await buttons[2]!.trigger("click"); // 保存
    expect(runMenuAction).toHaveBeenCalledWith("file.save");
  });

  it("无工具阶段显示阶段引导文案", async () => {
    gotoStage("mesh");
    const wrapper = mount(StageRibbon, { global: { plugins: [pinia] } });
    expect(wrapper.findAll("button").length).toBe(0);
    expect(wrapper.text()).toContain(STAGE_HINTS.mesh);
  });

  it("禁用按钮不可点击", async () => {
    gotoStage("results");
    const wrapper = mount(StageRibbon, { global: { plugins: [pinia] } });
    const exportBtn = wrapper.findAll("button")[0]!;
    expect(exportBtn.attributes("disabled")).toBeDefined();
    await exportBtn.trigger("click");
    expect(runMenuAction).not.toHaveBeenCalled();
  });
});
