import { beforeEach, describe, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import JobsPanel from "../../../../src-web/views/jobs/JobsPanel.vue";
import { useAppStore } from "../../../../src-web/stores/app";
import { useJobsStore } from "../../../../src-web/stores/jobs";
import { useDependenciesStore } from "../../../../src-web/stores/dependencies";
import { useVmStore } from "../../../../src-web/stores/vm";
import { probeOpenfoam } from "../../../../src-web/api/solver";
import { cancelJob, listJobs, submitJob } from "../../../../src-web/api/jobs";
import type { Job } from "../../../../src-web/types";

vi.mock("../../../../src-web/api/solver", () => ({
  probeOpenfoam: vi.fn(),
}));
vi.mock("../../../../src-web/api/jobs", () => ({
  submitJob: vi.fn(),
  cancelJob: vi.fn(),
  listJobs: vi.fn(),
}));
// happy-dom 无 Tauri 运行时：jobs store 构造 Channel 需要桩。
vi.mock("@tauri-apps/api/core", () => ({
  Channel: class {
    onmessage: ((message: string) => void) | null = null;
  },
}));

function makeJob(overrides: Partial<Job> = {}): Job {
  return {
    id: "job-1",
    studyId: null,
    caseDir: "/tmp/case",
    cores: 4,
    status: "running",
    createdMs: 0,
    startedMs: null,
    finishedMs: null,
    lastTimeS: null,
    message: null,
    ...overrides,
  };
}

function findButton(wrapper: ReturnType<typeof mount>, label: string) {
  const found = wrapper.findAll("button").find((candidate) => candidate.text() === label);
  if (found === undefined) {
    throw new Error(`找不到按钮：${label}`);
  }
  return found;
}

describe("JobsPanel 环境探测行", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    vi.resetAllMocks();
    vi.mocked(listJobs).mockResolvedValue([]);
    vi.mocked(probeOpenfoam).mockResolvedValue({
      openfoam: true,
      solver: true,
      hint: "OpenFOAM 环境就绪。",
    });
  });

  it("T54：求解环境更新未部署 → 顶部横幅提示；一致则无", async () => {
    const deps = useDependenciesStore();
    const vm = useVmStore();
    deps.downloadedFiles = {
      moldingfoam: {
        fileName: "moldingFoam.tar.xz",
        sizeBytes: 1,
        downloadedAtMs: 0,
        extractDir: null,
        releaseTag: "v0.2.0",
      },
    };
    vm.deployedReleaseTag = "v0.1.1";
    const wrapper = mount(JobsPanel, { global: { plugins: [pinia] } });
    expect(wrapper.text()).toContain("有更新未部署");
    expect(wrapper.text()).toContain("部署到虚拟机");

    vm.deployedReleaseTag = "v0.2.0";
    await wrapper.vm.$nextTick();
    expect(wrapper.text()).not.toContain("有更新未部署");
  });

  it("探测完成前显示探测中文案", () => {
    // 永不落定的探测 promise：维持「探测中」占位。
    vi.mocked(probeOpenfoam).mockReturnValue(new Promise(() => {}));
    const wrapper = mount(JobsPanel, { global: { plugins: [pinia] } });
    expect(wrapper.text()).toContain("正在探测 OpenFOAM 环境…");
  });

  it("工具链与求解器齐备显示绿色就绪提示", async () => {
    vi.mocked(probeOpenfoam).mockResolvedValue({
      openfoam: true,
      solver: true,
      hint: "OpenFOAM 环境就绪。",
    });
    const wrapper = mount(JobsPanel, { global: { plugins: [pinia] } });
    await vi.waitFor(() => expect(wrapper.text()).toContain("OpenFOAM 环境就绪。"));
    const hint = wrapper.findAll("p").find((p) => p.text() === "OpenFOAM 环境就绪。");
    expect(hint?.classes()).toContain("text-emerald-400");
    expect(hint?.classes()).not.toContain("text-xs");
  });

  it("缺少模块化求解器显示琥珀提示", async () => {
    vi.mocked(probeOpenfoam).mockResolvedValue({
      openfoam: true,
      solver: false,
      hint: "缺少 foamRun，请升级 OpenFOAM。",
    });
    const wrapper = mount(JobsPanel, { global: { plugins: [pinia] } });
    await vi.waitFor(() => expect(wrapper.text()).toContain("缺少 foamRun"));
    const hint = wrapper.findAll("p").find((p) => p.text()!.includes("缺少 foamRun"));
    expect(hint?.classes()).toContain("text-amber-400");
  });

  it("缺少 OpenFOAM 本体显示琥珀提示", async () => {
    vi.mocked(probeOpenfoam).mockResolvedValue({
      openfoam: false,
      solver: false,
      hint: "未检测到 OpenFOAM。",
    });
    const wrapper = mount(JobsPanel, { global: { plugins: [pinia] } });
    await vi.waitFor(() => expect(wrapper.text()).toContain("未检测到 OpenFOAM。"));
    const hint = wrapper.findAll("p").find((p) => p.text() === "未检测到 OpenFOAM。");
    expect(hint?.classes()).toContain("text-amber-400");
  });
});

describe("JobsPanel 作业列表与提交", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
    vi.resetAllMocks();
    vi.mocked(probeOpenfoam).mockResolvedValue({ openfoam: true, solver: true, hint: "就绪。" });
    vi.mocked(listJobs).mockResolvedValue([]);
  });

  it("无作业时显示占位", async () => {
    const wrapper = mount(JobsPanel, { global: { plugins: [pinia] } });
    expect(wrapper.text()).toContain("暂无作业。");
  });

  it("作业行渲染状态文案、目录与时间步", () => {
    const jobs = useJobsStore();
    jobs.jobs = [
      makeJob({ id: "job-a", status: "queued" }),
      makeJob({ id: "job-b", status: "running", lastTimeS: 1.5 }),
      makeJob({ id: "job-c", status: "done" }),
      makeJob({ id: "job-d", status: "failed" }),
      makeJob({ id: "job-e", status: "cancelled" }),
    ];
    const wrapper = mount(JobsPanel, { global: { plugins: [pinia] } });
    expect(wrapper.text()).toContain("job-a");
    expect(wrapper.text()).toContain("排队中");
    expect(wrapper.text()).toContain("运行中");
    expect(wrapper.text()).toContain("已完成");
    expect(wrapper.text()).toContain("失败");
    expect(wrapper.text()).toContain("已取消");
    expect(wrapper.text()).toContain("t = 1.50 s");
  });

  it("空 case 目录不提交", async () => {
    const wrapper = mount(JobsPanel, { global: { plugins: [pinia] } });
    await findButton(wrapper, "提交作业").trigger("click");
    expect(submitJob).not.toHaveBeenCalled();
  });

  it("提交作业：目录去空白、核数取输入值", async () => {
    vi.mocked(submitJob).mockResolvedValue(makeJob({ id: "job-9" }));
    const wrapper = mount(JobsPanel, { global: { plugins: [pinia] } });
    const inputs = wrapper.findAll("input");
    await inputs[0]!.setValue("  /tmp/case  ");
    await inputs[1]!.setValue("4");
    await findButton(wrapper, "提交作业").trigger("click");
    await vi.waitFor(() =>
      expect(submitJob).toHaveBeenCalledWith("/tmp/case", 4, null, expect.anything()),
    );
  });

  it("核数留空回落 2", async () => {
    vi.mocked(submitJob).mockResolvedValue(makeJob());
    const wrapper = mount(JobsPanel, { global: { plugins: [pinia] } });
    await wrapper.findAll("input")[0]!.setValue("/tmp/case");
    await findButton(wrapper, "提交作业").trigger("click");
    await vi.waitFor(() =>
      expect(submitJob).toHaveBeenCalledWith("/tmp/case", 2, null, expect.anything()),
    );
  });

  it("提交失败设置全局错误", async () => {
    vi.mocked(submitJob).mockRejectedValue(new Error("调度器不可用"));
    const wrapper = mount(JobsPanel, { global: { plugins: [pinia] } });
    await wrapper.findAll("input")[0]!.setValue("/tmp/case");
    await findButton(wrapper, "提交作业").trigger("click");
    await vi.waitFor(() => expect(useAppStore().error?.message).toBe("调度器不可用"));
  });

  it("作业日志尾部渲染最后 8 行，无日志作业不渲染日志块", async () => {
    const jobs = useJobsStore();
    jobs.jobs = [makeJob({ id: "job-with-log" }), makeJob({ id: "job-silent" })];
    jobs.jobLogs = {
      "job-with-log": Array.from({ length: 10 }, (_, i) => `line-${i}`),
    };
    const wrapper = mount(JobsPanel, { global: { plugins: [pinia] } });
    const pre = wrapper.find("pre");
    expect(pre.exists()).toBe(true);
    // 只取最后 8 行。
    expect(pre.text()).toBe(Array.from({ length: 8 }, (_, i) => `line-${i + 2}`).join("\n"));
  });

  it("排队与运行中的作业可取消，结束态禁用", async () => {
    const jobs = useJobsStore();
    jobs.jobs = [
      makeJob({ id: "job-run", status: "running" }),
      makeJob({ id: "job-done", status: "done" }),
    ];
    vi.mocked(cancelJob).mockResolvedValue(undefined);
    const wrapper = mount(JobsPanel, { global: { plugins: [pinia] } });
    const buttons = wrapper.findAll("button").filter((button) => button.text() === "取消");
    expect(buttons[0]!.attributes("disabled")).toBeUndefined();
    expect(buttons[1]!.attributes("disabled")).toBeDefined();
    vi.mocked(listJobs).mockResolvedValue([makeJob({ id: "job-run", status: "cancelled" })]);
    await buttons[0]!.trigger("click");
    await vi.waitFor(() => expect(cancelJob).toHaveBeenCalledWith("job-run"));
  });

  it("取消失败设置全局错误", async () => {
    const jobs = useJobsStore();
    jobs.jobs = [makeJob({ id: "job-x", status: "queued" })];
    vi.mocked(cancelJob).mockRejectedValue(new Error("取消失败"));
    const wrapper = mount(JobsPanel, { global: { plugins: [pinia] } });
    await findButton(wrapper, "取消").trigger("click");
    await vi.waitFor(() => expect(useAppStore().error?.message).toBe("取消失败"));
  });

  it("刷新按钮重新拉取作业列表", async () => {
    const wrapper = mount(JobsPanel, { global: { plugins: [pinia] } });
    await findButton(wrapper, "刷新").trigger("click");
    await vi.waitFor(() => expect(listJobs).toHaveBeenCalledTimes(1));
  });
});
