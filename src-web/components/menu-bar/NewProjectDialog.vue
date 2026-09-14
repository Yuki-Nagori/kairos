<script setup lang="ts">
/**
 * 新建项目对话框：项目名（决定工程目录）与文件名（决定 .kairos 文件），
 * 文件名默认跟随项目名。逻辑见 useNewProjectDialog。
 */
import { useNewProjectDialog } from "./useNewProjectDialog";
import TextInput from "../ui/UiTextInput.vue";
import UiButton from "../ui/UiButton.vue";

const { open, name, fileName, error, hide, markFileNameEdited, submit } = useNewProjectDialog();
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
        <p class="mt-1 text-[11px] leading-4 text-zinc-500">
          工程保存在「文档 / kairos / {{ name.trim() === "" ? "<项目名>" : name.trim() }} /
          {{ fileName.trim() === "" ? name.trim() || "<文件名>" : fileName.trim() }}.kairos」。
        </p>
        <label class="mt-4 flex flex-col gap-1">
          <span class="text-xs text-zinc-400">项目名</span>
          <TextInput v-model="name" type="text" placeholder="例如：控制器支架" />
        </label>
        <label class="mt-3 flex flex-col gap-1">
          <span class="text-xs text-zinc-400">文件名</span>
          <TextInput
            v-model="fileName"
            type="text"
            :placeholder="name.trim() === '' ? '默认与项目名相同' : name.trim()"
            @input="markFileNameEdited()"
          />
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
