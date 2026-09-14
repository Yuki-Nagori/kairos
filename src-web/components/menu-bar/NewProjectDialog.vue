<script setup lang="ts">
/**
 * 新建项目对话框：工作区路径（默认「文档 / kairos」，可改 / 可选目录）与项目名；
 * 工程文件与项目名同名。逻辑见 useNewProjectDialog。
 */
import { computed, toRef } from "vue";
import { useNewProjectDialog } from "./useNewProjectDialog";
import TextInput from "../ui/UiTextInput.vue";
import UiButton from "../ui/UiButton.vue";

const { open, workspace, name, error, hide, browseWorkspace, submit } = useNewProjectDialog();
const workspaceText = toRef(() => workspace.value.trim().replace(/[/\\]+$/, ""));
const nameText = toRef(() => name.value.trim());

/** 落点预览：工作区 + 项目名 → 工程目录与工程文件。 */
const placement = computed(() => {
  const base = workspaceText.value === "" ? "<工作区>" : workspaceText.value;
  const project = nameText.value === "" ? "<项目名>" : nameText.value;
  return `${base}/${project}/${project}.kairos`;
});
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="fixed inset-0 z-60 flex items-center justify-center bg-black/50"
      @click.self="hide()"
    >
      <div class="w-96 rounded-xl border border-zinc-700 bg-zinc-900 p-5 shadow-2xl">
        <h2 class="text-sm font-semibold text-zinc-100">新建项目</h2>
        <p class="mt-1 text-[11px] leading-4 text-zinc-500">工程保存在 {{ placement }}</p>
        <label class="mt-4 flex flex-col gap-1">
          <span class="text-xs text-zinc-400">工作区路径</span>
          <span class="flex items-center gap-2">
            <TextInput
              v-model="workspace"
              type="text"
              placeholder="默认：文档 / kairos"
              class="flex-1"
            />
            <UiButton @click="browseWorkspace()">选择…</UiButton>
          </span>
        </label>
        <label class="mt-3 flex flex-col gap-1">
          <span class="text-xs text-zinc-400">项目名</span>
          <TextInput v-model="name" type="text" placeholder="例如：控制器支架" />
        </label>
        <p v-if="error !== null" class="mt-3 text-xs text-red-300">{{ error }}</p>
        <div class="mt-4 flex justify-end gap-2">
          <UiButton @click="hide()">取消</UiButton>
          <UiButton variant="primary" @click="submit()">创建</UiButton>
        </div>
      </div>
    </div>
  </Teleport>
</template>
