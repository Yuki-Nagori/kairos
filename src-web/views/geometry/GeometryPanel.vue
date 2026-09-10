<script setup lang="ts">
/** 几何面板：逻辑见 useGeometryPanel。 */
import { useGeometryPanel } from "./useGeometryPanel";
import Card from "../../components/ui/UiCard.vue";
import TextInput from "../../components/ui/UiTextInput.vue";
import UiButton from "../../components/ui/UiButton.vue";

const {
  geometry,
  working,
  rows,
  issueText,
  isClean,
  statsText,
  generate,
  reportText,
  onImport,
  onSample,
} = useGeometryPanel();
</script>

<template>
  <Card title="几何">
    <div class="flex flex-wrap items-center gap-2">
      <UiButton variant="primary" :disabled="working" @click="onImport">导入 STL</UiButton>
      <UiButton @click="onSample">导入样例</UiButton>
    </div>

    <div class="space-y-2">
      <p v-if="geometry.geometries.length === 0" class="text-xs text-zinc-500">
        尚未导入几何。支持二进制 / ASCII STL。
      </p>
      <div
        v-for="row in rows"
        :key="row.geometry.geometryId"
        class="flex flex-wrap items-center gap-x-4 gap-y-1 rounded-lg border border-zinc-800 px-3 py-2 text-xs"
      >
        <span class="font-semibold text-zinc-200">{{ row.geometry.fileName }}</span>
        <span class="text-zinc-500">{{ statsText(row.geometry) }}</span>
        <p :class="isClean(row.geometry) ? 'text-emerald-400' : 'text-amber-400'">
          {{ issueText(row.geometry.issues) }}
        </p>
        <UiButton
          variant="danger"
          :disabled="working"
          @click="geometry.removeGeometryById(row.geometry.geometryId)"
        >
          移除
        </UiButton>

        <div class="flex w-full flex-wrap items-center gap-2 border-t border-zinc-800 pt-2">
          <TextInput
            v-model="row.form.size"
            type="number"
            placeholder="目标尺寸"
            class="w-28"
            step="any"
            min="0"
          />
          <select v-model="row.form.engine" class="text-xs">
            <option value="voxel">体素</option>
            <option value="gmsh">Gmsh</option>
          </select>
          <UiButton :disabled="working" @click="generate(row.geometry)">生成体积网格</UiButton>
          <p class="text-xs text-zinc-500">
            {{ reportText(geometry.meshReports[row.geometry.geometryId]) }}
          </p>
        </div>
      </div>
    </div>
  </Card>
</template>
