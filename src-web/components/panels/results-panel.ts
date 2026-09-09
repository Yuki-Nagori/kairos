import { appStore, loadField, loadResultsCatalog } from "../../state";
import { button, card, hint, textInput } from "../ui";
import { minMax } from "../../lib/stats";
/** 结果面板：扫描 case 结果目录、查看时间步与场统计（完整视口见 T14）。 */
export function createResultsPanel(): HTMLElement {
  const { root, body } = card("结果");

  const scanForm = document.createElement("div");
  scanForm.className = "flex flex-wrap items-center gap-2";
  const dirInput = textInput("OpenFOAM case 目录路径", "flex-1 min-w-48");
  const scanButton = button("扫描结果", "primary");
  scanForm.append(dirInput, scanButton);

  const timesBox = document.createElement("div");
  timesBox.className = "space-y-1";

  const statsBox = document.createElement("div");
  statsBox.className = "space-y-1";

  scanButton.addEventListener("click", () => {
    const caseDir = dirInput.value.trim();
    if (caseDir) {
      void loadResultsCatalog(caseDir);
    }
  });

  function render(): void {
    const { busy, loadedField } = appStore.get();
    const working = busy !== null;
    scanButton.disabled = working;

    timesBox.replaceChildren();
    const catalog = appStore.get().resultCatalog;
    if (catalog === null) {
      timesBox.append(hint("尚未扫描结果目录。"));
    } else if (catalog.times.length === 0) {
      timesBox.append(hint("结果目录中未发现时间步。"));
    } else {
      const table = document.createElement("table");
      table.className = "w-full text-xs";
      const header = document.createElement("tr");
      for (const text of ["时间步", "时间 (s)", "可用场"]) {
        const th = document.createElement("th");
        th.className = "border-b border-zinc-700 px-2 py-1 text-left text-zinc-400";
        th.textContent = text;
        header.append(th);
      }
      const head = document.createElement("thead");
      head.append(header);
      const bodyEl = document.createElement("tbody");
      for (const time of catalog.times) {
        const tr = document.createElement("tr");
        const dirCell = document.createElement("td");
        dirCell.className = "px-2 py-1 text-zinc-300";
        dirCell.textContent = time.dirName;
        const loadButton = document.createElement("button");
        loadButton.type = "button";
        loadButton.className = "text-emerald-400 hover:underline";
        loadButton.textContent = "加载 T 场";
        loadButton.addEventListener("click", () => {
          void loadField(catalog.caseDir, time.dirName, "T");
        });
        dirCell.append(loadButton);
        const timeCell = document.createElement("td");
        timeCell.className = "px-2 py-1 text-zinc-400";
        timeCell.textContent = time.timeS.toFixed(3);
        const fieldsCell = document.createElement("td");
        fieldsCell.className = "px-2 py-1 text-zinc-400";
        fieldsCell.textContent = time.fields.join(", ");
        tr.append(dirCell, timeCell, fieldsCell);
        bodyEl.append(tr);
      }
      table.append(head, bodyEl);
      timesBox.append(table);
    }

    statsBox.replaceChildren();
    if (loadedField) {
      const values = loadedField.values;
      const { min, max } =
        values.length > 0 ? minMax(values) : { min: Number.NaN, max: Number.NaN };
      const line = document.createElement("p");
      line.className = "text-xs text-zinc-400";
      line.textContent = `已加载 ${loadedField.field} @ ${loadedField.timeDir}${loadedField.isMagnitude ? "（模量）" : ""}：${values.length} 个值，min ${min.toFixed(3)} / max ${max.toFixed(3)}`;
      const complete = document.createElement("p");
      complete.className = loadedField.complete
        ? "text-xs text-emerald-400"
        : "text-xs text-amber-400";
      complete.textContent = loadedField.complete ? "结果完整" : "结果不完整（求解中途取消）";
      statsBox.append(line, complete);
    }
  }

  // 派生场（T31）：对已加载场做标量运算生成新场，图表与视口即时可用。
  const deriveSelect = document.createElement("select");
  deriveSelect.className = "flex-1 min-w-0 text-xs";
  for (const [value, label] of [
    ["normalize", "归一化 (0–1)"],
    ["threshold", "阈值掩码（中位幅值）"],
  ] as const) {
    const option = document.createElement("option");
    option.value = value;
    option.textContent = label;
    deriveSelect.append(option);
  }
  const deriveButton = button("生成派生场", "ghost");
  deriveButton.disabled = true;
  const deriveRow = document.createElement("div");
  deriveRow.className = "flex items-center gap-2";
  deriveRow.append(deriveSelect, deriveButton);

  deriveButton.addEventListener("click", () => {
    const source = appStore.get().loadedField;
    if (!source || source.values.length === 0) {
      return;
    }
    const values = source.values;
    let derived: number[];
    let suffix: string;
    if (deriveSelect.value === "threshold") {
      const { min, max } = minMax(values);
      const threshold = (min + max) / 2;
      derived = values.map((v) => (v >= threshold ? 1 : 0));
      suffix = "阈值掩码";
    } else {
      const { min, max } = minMax(values);
      const range = max - min;
      derived = range > 0 ? values.map((v) => (v - min) / range) : values.map(() => 0);
      suffix = "归一化";
    }
    appStore.set({
      loadedField: {
        ...source,
        field: `${source.field} · ${suffix}`,
        isMagnitude: false,
        values: derived,
        complete: source.complete,
      },
    });
  });

  function renderDerive(): void {
    const loaded = appStore.get().loadedField !== null;
    deriveButton.disabled = !loaded;
    deriveSelect.disabled = !loaded;
  }

  body.append(scanForm, timesBox, statsBox, deriveRow);
  appStore.subscribe(() => {
    render();
    renderDerive();
  });
  render();
  renderDerive();
  return root;
}
