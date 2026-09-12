<script setup lang="ts">
/** 求解作业面板：逻辑见 useJobsPanel。 */
import { useJobsPanel } from "./useJobsPanel";
import Card from "../../components/ui/UiCard.vue";
import TextInput from "../../components/ui/UiTextInput.vue";
import UiButton from "../../components/ui/UiButton.vue";

const {
  app,
  jobsStore,
  pendingDeployText,
  caseDir,
  cores,
  submitJob,
  envHint,
  envClass,
  STATUS_LABEL,
  STATUS_CLASS,
  jobLogsTail,
} = useJobsPanel();
</script>

<template>
  <Card title="求解作业">
    <!-- 求解环境「已下载新版本但 VM 未部署」提醒 -->
    <p
      v-if="pendingDeployText"
      class="mb-2 rounded-lg border border-amber-500/50 bg-amber-500/10 px-3 py-2 text-xs text-amber-300"
    >
      {{ pendingDeployText }}
    </p>
    <p :class="envClass">{{ envHint }}</p>
    <div class="flex flex-wrap items-center gap-2">
      <TextInput
        v-model="caseDir"
        type="text"
        placeholder="case 目录路径"
        class="flex-1 min-w-48"
      />
      <TextInput v-model="cores" type="number" placeholder="2" class="w-20" min="1" />
      <UiButton variant="primary" :disabled="app.busy !== null" @click="submitJob">
        提交作业
      </UiButton>
      <UiButton :disabled="app.busy !== null" @click="jobsStore.refreshJobs()">刷新</UiButton>
    </div>
    <div class="space-y-2">
      <p v-if="jobsStore.jobs.length === 0" class="text-xs text-zinc-500">暂无作业。</p>
      <template v-else>
        <template v-for="job in jobsStore.jobs" :key="job.id">
          <div
            class="flex flex-wrap items-center gap-x-4 gap-y-1 rounded-lg border border-zinc-800 px-3 py-2 text-xs"
          >
            <span class="font-mono text-zinc-300">{{ job.id }}</span>
            <!-- 状态行只渲染文字，无状态圆点（有意省略）。 -->
            <span class="flex items-center gap-1.5" :class="STATUS_CLASS[job.status]">
              {{ STATUS_LABEL[job.status] }}
            </span>
            <span class="truncate text-zinc-500">{{ job.caseDir }}</span>
            <span class="text-zinc-500">
              {{ job.lastTimeS !== null ? `t = ${job.lastTimeS.toFixed(2)} s` : "" }}
            </span>
            <UiButton
              variant="danger"
              :disabled="app.busy !== null || (job.status !== 'queued' && job.status !== 'running')"
              @click="jobsStore.cancelJob(job.id)"
            >
              取消
            </UiButton>
          </div>
          <!-- 求解日志尾部（环形缓冲的最后 8 行），运行中与结束后都可查看。 -->
          <pre
            v-if="jobLogsTail(job.id).length > 0"
            class="max-h-32 overflow-y-auto whitespace-pre-wrap break-all rounded border border-zinc-800 bg-zinc-950 px-2 py-1 text-[10px] leading-4 text-zinc-500"
            >{{ jobLogsTail(job.id).slice(-8).join("\n") }}</pre>
        </template>
      </template>
    </div>
  </Card>
</template>
