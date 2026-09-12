<script setup lang="ts">
/** 方案任务窗格：任务序列（六态图标 / 失败阻断 / 双击直达编辑阶段）
 *  + 底部分析序列与提交表单。逻辑见 useStudyTasks。 */
import { useStudyTasks, TASK_STATE_META } from "./useStudyTasks";
import Card from "../../components/ui/UiCard.vue";
import Dropdown from "../../components/ui/UiDropdown.vue";
import TextInput from "../../components/ui/UiTextInput.vue";
import UiButton from "../../components/ui/UiButton.vue";

const {
  tasks,
  submitDisabled,
  openTask,
  submit,
  stage,
  cores,
  menu,
  openMenu,
  closeMenu,
  taskMenuCommands,
} = useStudyTasks();
</script>

<template>
  <Card title="方案任务" collapsible>
    <ol class="space-y-1">
      <li v-for="task in tasks" :key="task.id">
        <button
          type="button"
          class="flex w-full gap-2.5 rounded-md px-1 py-1 text-left transition-colors hover:bg-zinc-800/50"
          :title="`双击打开「${task.label}」编辑，右键更多命令`"
          @dblclick="openTask(task)"
          @contextmenu.prevent="openMenu(task, $event)"
        >
          <!-- 六态图标：✓ 成功 / ! 警告 / ✕ 失败 / ⧖ 排队 / ⟳ 执行中 / 空 未开始 -->
          <span
            class="mt-0.5 flex size-3.5 shrink-0 items-center justify-center rounded text-[9px] font-semibold"
            :class="TASK_STATE_META[task.state].cls"
            >{{ TASK_STATE_META[task.state].icon }}</span
          >
          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2">
              <span
                class="text-xs"
                :class="
                  task.state === 'failed'
                    ? 'text-red-400'
                    : task.state === 'done' || task.state === 'warning'
                      ? 'text-zinc-500'
                      : task.state === 'running'
                        ? 'text-zinc-100'
                        : 'text-zinc-400'
                "
                >{{ task.label }}</span
              >
              <span
                v-if="task.detail"
                class="ml-auto truncate text-[10px] tabular-nums text-zinc-500"
                >{{ task.detail }}</span
              >
            </div>
            <p
              v-if="task.state === 'blocked' && task.blockReason"
              class="text-[11px] leading-4 text-zinc-600"
            >
              {{ task.blockReason }}
            </p>
            <p
              v-else-if="task.state !== 'done' && task.hint"
              class="text-[11px] leading-4 text-zinc-500"
            >
              {{ task.hint }}
            </p>
          </div>
        </button>
      </li>
    </ol>
    <!-- 任务右键菜单：定位在指针处，点击任意处 / Esc 关闭（监听在 composable） -->
    <div
      v-if="menu"
      class="fixed z-50 min-w-36 rounded-lg border border-zinc-700 bg-zinc-900 py-1 shadow-xl"
      :style="{ left: `${menu.x}px`, top: `${menu.y}px` }"
      @click.stop
    >
      <button
        v-for="item in taskMenuCommands(menu.task)"
        :key="item.label"
        type="button"
        class="block w-full px-3 py-1.5 text-left text-xs transition-colors hover:bg-zinc-800"
        :class="item.danger ? 'text-red-400' : 'text-zinc-300'"
        @click="
          item.run();
          closeMenu();
        "
      >
        {{ item.label }}
      </button>
    </div>
    <div class="mt-2 flex flex-wrap items-center gap-2 border-t border-zinc-800 pt-2">
      <Dropdown v-model="stage">
        <option value="fill">填充</option>
        <option value="fill_pack">填充 + 保压</option>
        <option value="fill_pack_cool">填充 + 保压 + 冷却</option>
      </Dropdown>
      <p class="text-xs text-zinc-500">核数</p>
      <TextInput
        v-model="cores"
        type="number"
        placeholder="2"
        class="w-20"
        min="1"
        title="并行核数"
      />
      <UiButton variant="primary" :disabled="submitDisabled" @click="submit">
        提交求解作业
      </UiButton>
    </div>
  </Card>
</template>
