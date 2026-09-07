import {
  appStore,
  deleteMaterial,
  importMaterials,
  exportMaterials,
  copyMaterialToCustom,
  assignMaterial,
} from "../state";
import type { Material } from "../types";
import { button, card, hint } from "./ui";

/** 材料库面板：内置示例材料 + 自定义材料的浏览、详情、导入导出与复制。 */
export function createMaterialsPanel(): HTMLElement {
  const { root, body } = card("材料库");

  let selectedId: string | null = null;

  const actions = document.createElement("div");
  actions.className = "flex flex-wrap items-center gap-2";
  const importButton = button("导入 JSON");
  const exportButton = button("导出自定义");
  const copyButton = button("复制为自定义");
  const deleteButton = button("删除", "danger");
  const useButton = button("用于当前研究");
  actions.append(importButton, exportButton, copyButton, useButton, deleteButton);
  useButton.addEventListener("click", () => {
    if (selectedId) {
      assignMaterial(selectedId);
    }
  });

  const listBox = document.createElement("div");
  listBox.className = "flex gap-4";
  const builtinList = document.createElement("div");
  builtinList.className = "w-44 shrink-0 space-y-1";
  const customList = document.createElement("div");
  customList.className = "w-44 shrink-0 space-y-1";

  const listColumn = (title: string, box: HTMLElement): HTMLElement => {
    const wrapper = document.createElement("div");
    const heading = document.createElement("p");
    heading.className = "mb-1 text-xs font-semibold text-zinc-400";
    heading.textContent = title;
    wrapper.append(heading, box);
    return wrapper;
  };
  listBox.append(listColumn("内置示例", builtinList), listColumn("自定义", customList));

  const detail = document.createElement("div");
  detail.className = "min-w-0 flex-1 space-y-2";

  importButton.addEventListener("click", () => void importMaterials());
  exportButton.addEventListener("click", () => void exportMaterials());
  copyButton.addEventListener("click", () => {
    if (selectedId) {
      void copyMaterialToCustom(selectedId);
    }
  });
  deleteButton.addEventListener("click", () => {
    if (selectedId) {
      void deleteMaterial(selectedId);
      selectedId = null;
    }
  });

  function materialButton(material: Material): HTMLButtonElement {
    const element = document.createElement("button");
    element.type = "button";
    element.textContent = `${material.family} · ${material.name}`;
    element.dataset.materialId = material.id;
    element.className =
      "block w-full truncate rounded-lg border border-zinc-700 px-2 py-1.5 text-left text-xs hover:border-emerald-500";
    element.addEventListener("click", () => {
      selectedId = material.id;
      render();
    });
    return element;
  }

  function paramTable(rows: [string, string][]): HTMLTableElement {
    const table = document.createElement("table");
    table.className = "text-xs";
    for (const [key, value] of rows) {
      const tr = document.createElement("tr");
      const keyCell = document.createElement("td");
      keyCell.className = "pr-4 py-0.5 text-zinc-500";
      keyCell.textContent = key;
      const valueCell = document.createElement("td");
      valueCell.className = "py-0.5 font-mono text-zinc-300";
      valueCell.textContent = value;
      tr.append(keyCell, valueCell);
      table.append(tr);
    }
    return table;
  }

  function valueTable(table: [number, number][], unit: string): HTMLTableElement {
    const rows: [string, string][] = table.map(([temperature, value]) => [
      `${temperature} K`,
      `${value} ${unit}`,
    ]);
    return paramTable(rows);
  }

  function renderDetail(material: Material | undefined): void {
    detail.replaceChildren();
    if (!material) {
      detail.append(hint("选择左侧材料查看参数。"));
      return;
    }
    const heading = document.createElement("p");
    heading.className = "text-sm font-semibold text-zinc-200";
    heading.textContent = `${material.name}（${material.manufacturer}）`;
    detail.append(heading);

    const rheologyTitle = document.createElement("p");
    rheologyTitle.className = "text-xs font-semibold text-zinc-400";
    rheologyTitle.textContent = "Cross-WLF 黏度";
    detail.append(
      rheologyTitle,
      paramTable([
        ["n", String(material.rheology.n)],
        ["τ*", `${material.rheology.tauStar} Pa`],
        ["D1", `${material.rheology.d1} Pa·s`],
        ["D2", `${material.rheology.d2} K`],
        ["D3", `${material.rheology.d3} K/Pa`],
        ["A1", String(material.rheology.a1)],
        ["A2", `${material.rheology.a2} K`],
      ]),
    );

    const pvtTitle = document.createElement("p");
    pvtTitle.className = "text-xs font-semibold text-zinc-400";
    pvtTitle.textContent = "Tait PVT";
    detail.append(
      pvtTitle,
      paramTable([
        ["b1m", `${material.pvt.b1m} m³/kg`],
        ["b1s", `${material.pvt.b1s} m³/kg`],
        ["b2m", `${material.pvt.b2m} m³/(kg·K)`],
        ["b2s", `${material.pvt.b2s} m³/(kg·K)`],
        ["b3", `${material.pvt.b3} Pa`],
        ["b4m", `${material.pvt.b4m} 1/K`],
        ["b4s", `${material.pvt.b4s} 1/K`],
        ["b5", `${material.pvt.b5} K`],
      ]),
    );

    const heatTitle = document.createElement("p");
    heatTitle.className = "text-xs font-semibold text-zinc-400";
    heatTitle.textContent = "比热 Cp";
    detail.append(heatTitle, valueTable(material.specificHeat, "J/(kg·K)"));
    const condTitle = document.createElement("p");
    condTitle.className = "text-xs font-semibold text-zinc-400";
    condTitle.textContent = "导热系数 λ";
    detail.append(condTitle, valueTable(material.conductivity, "W/(m·K)"));

    if (material.mechanics) {
      const mechTitle = document.createElement("p");
      mechTitle.className = "text-xs font-semibold text-zinc-400";
      mechTitle.textContent = "力学（预留）";
      detail.append(
        mechTitle,
        paramTable([
          ["E", `${material.mechanics.elasticModulus} Pa`],
          ["ν", String(material.mechanics.poissonRatio)],
        ]),
      );
    }

    const note = hint(material.dataNote);
    detail.append(note);
  }

  function render(): void {
    const { materials, busy } = appStore.get();
    const working = busy !== null;

    builtinList.replaceChildren();
    customList.replaceChildren();
    for (const material of materials.builtin) {
      builtinList.append(materialButton(material));
    }
    for (const material of materials.custom) {
      customList.append(materialButton(material));
    }
    if (materials.builtin.length === 0) {
      builtinList.append(hint("加载中…"));
    }
    if (materials.custom.length === 0) {
      customList.append(hint("无自定义材料。"));
    }

    const selected =
      [...materials.builtin, ...materials.custom].find((m) => m.id === selectedId) ??
      materials.builtin[0];
    if (selected && selectedId !== selected.id) {
      selectedId = selected.id;
    }
    renderDetail(selected);

    deleteButton.disabled =
      !selectedId || !materials.custom.some((m) => m.id === selectedId) || working;
    copyButton.disabled = !selectedId || working;
    importButton.disabled = working;
    exportButton.disabled = materials.custom.length === 0 || working;

    root.querySelectorAll("button[data-material-id]").forEach((element) => {
      const active = (element as HTMLElement).dataset.materialId === selectedId;
      element.classList.toggle("border-emerald-500", active);
      element.classList.toggle("bg-emerald-500/10", active);
    });
  }

  body.append(actions, listBox, detail);
  render();
  appStore.subscribe(render);
  return root;
}
