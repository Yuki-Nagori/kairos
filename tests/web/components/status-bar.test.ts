import { beforeEach, describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import type { Pinia } from "pinia";
import StatusBar from "../../../src-web/components/status-bar/StatusBar.vue";
import { useAppStore } from "../../../src-web/stores/app";
import { useJobsStore } from "../../../src-web/stores/jobs";
import { useVmStore } from "../../../src-web/stores/vm";
import type { Job, SystemInfo } from "../../../src-web/types";

const info: SystemInfo = { name: "kairos", version: "0.1.0", os: "macos" };

function jobFixture(overrides: Partial<Job> = {}): Job {
  return {
    id: "job-1",
    studyId: null,
    caseDir: "/case/job-1",
    cores: 2,
    status: "queued",
    createdMs: 1,
    startedMs: null,
    finishedMs: null,
    lastTimeS: null,
    message: null,
    ...overrides,
  };
}

describe("StatusBar 左侧状态三态优先级", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
  });

  it("空闲时显示 IPC 正常并在右侧显示版本与平台", () => {
    useAppStore().info = info;
    const wrapper = mount(StatusBar, { global: { plugins: [pinia] } });
    expect(wrapper.find("span").text()).toBe("● IPC 正常");
    expect(wrapper.find("span").classes()).toContain("text-emerald-400");
    expect(wrapper.text()).toContain("v0.1.0 · macos");
  });

  it("忙碌优先于版本信息", () => {
    const app = useAppStore();
    app.info = info;
    app.busy = "正在求解…";
    const wrapper = mount(StatusBar, { global: { plugins: [pinia] } });
    expect(wrapper.find("span").text()).toBe("正在求解…");
  });

  it("错误优先于忙碌与版本", () => {
    const app = useAppStore();
    app.info = info;
    app.busy = "正在求解…";
    app.error = { message: "迭代不收敛", info: false };
    const wrapper = mount(StatusBar, { global: { plugins: [pinia] } });
    expect(wrapper.find("span").text()).toBe("迭代不收敛");
  });

  it("IPC 提示型错误用中性色而非错误红", () => {
    const app = useAppStore();
    app.info = info;
    app.error = { message: "未连接 Tauri 运行时", info: true };
    const wrapper = mount(StatusBar, { global: { plugins: [pinia] } });
    expect(wrapper.find("span").text()).toBe("未连接 Tauri 运行时");
    expect(wrapper.find("span").classes()).toContain("text-zinc-400");
    expect(wrapper.find("span").classes()).not.toContain("text-red-400");
  });

  it("尚未拿到 system info 时右侧不渲染版本", () => {
    const wrapper = mount(StatusBar, { global: { plugins: [pinia] } });
    expect(wrapper.text()).not.toContain("v");
  });
});

describe("StatusBar 求解进度", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
  });

  it("运行中作业显示物理时间，空闲不显示", async () => {
    const jobsStore = useJobsStore();
    const wrapper = mount(StatusBar, { global: { plugins: [pinia] } });
    expect(wrapper.text()).not.toContain("求解进度");

    jobsStore.jobs = [jobFixture({ status: "running", lastTimeS: 0.841 })];
    await wrapper.vm.$nextTick();
    const progress = wrapper.findAll("span").find((span) => span.text().includes("求解进度"))!;
    expect(progress.text()).toBe("求解进度 · Time = 0.84 s");
  });

  it("非运行中或无时间的作业不显示进度", () => {
    const jobsStore = useJobsStore();
    jobsStore.jobs = [
      jobFixture({ id: "job-1", status: "done", lastTimeS: 2.5 }),
      jobFixture({ id: "job-2", status: "running", lastTimeS: null }),
    ];
    const wrapper = mount(StatusBar, { global: { plugins: [pinia] } });
    expect(wrapper.text()).not.toContain("求解进度");
  });
});

describe("StatusBar Shell 入口", () => {
  let pinia: Pinia;

  beforeEach(() => {
    pinia = createPinia();
    setActivePinia(pinia);
  });

  it("点击切换虚拟机面板显隐并更新文案", async () => {
    const wrapper = mount(StatusBar, { global: { plugins: [pinia] } });
    const vm = useVmStore();
    const button = wrapper.find("button");
    // 文字提示在左，shell 图标（内联 SVG）在右
    expect(button.text()).toContain("Shell 环境");
    expect(button.find("svg").exists()).toBe(true);
    expect(vm.vmPanelVisible).toBe(false);

    await button.trigger("click");
    expect(vm.vmPanelVisible).toBe(true);
    expect(button.text()).toContain("点击收起");

    await button.trigger("click");
    expect(vm.vmPanelVisible).toBe(false);
  });
});
