import { beforeEach, describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import LogTabs from "../../../src-web/components/LogTabs.vue";
import { useGeometryStore } from "../../../src-web/stores/geometry";
import { useJobsStore } from "../../../src-web/stores/jobs";
import { useVmStore } from "../../../src-web/stores/vm";
import type { Job, MeshingReport } from "../../../src-web/types";

const job: Job = {
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
};

const report: MeshingReport = {
  engine: "gmsh",
  nodeCount: 1200,
  elementCount: 5300,
  surfaceFaceCount: 900,
  totalVolume: 42.5,
  quality: { minEdgeRatio: 1.01, avgEdgeRatio: 1.7, maxEdgeRatio: 8.9, minVolume: 0.001 },
};

function tabs(wrapper: ReturnType<typeof mount>) {
  return wrapper.findAll("button");
}

describe("LogTabs", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
  });

  it("默认分析日志页，无日志时显示占位", () => {
    const wrapper = mount(LogTabs, { global: { plugins: [pinia] } });
    expect(tabs(wrapper).map((tab) => tab.text())).toEqual(["分析日志", "网格日志", "VM 终端"]);
    expect(wrapper.find("pre").text()).toBe("（暂无日志）");
  });

  it("分析日志取最新作业的日志尾部", () => {
    const jobs = useJobsStore();
    jobs.jobs = [job];
    jobs.jobLogs = { "job-1": ["a", "b"] };
    const wrapper = mount(LogTabs, { global: { plugins: [pinia] } });
    expect(wrapper.find("pre").text()).toBe("a\nb");
  });

  it("网格日志展示引擎与质量摘要", async () => {
    useGeometryStore().meshReports = { "geo-1": report };
    const wrapper = mount(LogTabs, { global: { plugins: [pinia] } });
    await tabs(wrapper)[1]!.trigger("click");
    expect(wrapper.find("pre").text()).toContain("引擎 gmsh · 1200 节点 / 5300 四面体");
    expect(wrapper.find("pre").text()).toContain("max 8.90");
  });

  it("VM 终端页展示 Shell 输出", async () => {
    useVmStore().vmShellLogs = ["$ ls", "case"];
    const wrapper = mount(LogTabs, { global: { plugins: [pinia] } });
    await tabs(wrapper)[2]!.trigger("click");
    expect(wrapper.find("pre").text()).toBe("$ ls\ncase");
  });

  it("激活标签高亮切换", async () => {
    const wrapper = mount(LogTabs, { global: { plugins: [pinia] } });
    await tabs(wrapper)[2]!.trigger("click");
    expect(tabs(wrapper)[2]!.classes()).toContain("text-emerald-400");
    expect(tabs(wrapper)[0]!.classes()).not.toContain("text-emerald-400");
  });
});
