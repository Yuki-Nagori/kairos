# T36 · moldingFoam 契约对接 + bundle 进虚拟机执行

- 阶段：Part 1（契约 case 重写）已完成；Part 2（bundle 进 VM 执行）待开工（设计见下）
- 依赖：T34（bundle 下载已落地）、T35（虚拟机适配层）
- 优先级：**P1**

## 目标

1. case 生成器切换到 moldingFoam `case-contract/` v1.1 字典布局，
   `SOLVER_MODULE` 切为 `moldingFoam`；
2. 下载到宿主的 bundle 自动传输进虚拟机，求解作业在 VM 内执行。

## Part 1 · case 生成重写（kairos-core services/openfoam.rs）

### 已核实的契约事实（~/eit/moldingFoam/case-contract/ v1.1）

- `system/controlDict`：`application foamRun; solver moldingFoam;
libs ("libmoldingFoam.so");` + `adjustTimeStep on; maxCo 0.5;
maxAlphaCo 0.10; deltaT 1e-05;`（验收函数对象不需要）；
- `constant/moldingDict`：`injection.meltTemperature`（K）、
  `packing.switchFraction`（-）、`packing.pressure`（table，相对 V/P
  切换时刻，首点须为大气压 1e5 防压力阶跃）、`cooling.ejectionTemperature`（K）+
  `cooling.releasePressure`；
- `constant/momentumTransport`：`laminar` + `generalisedNewtonian` +
  `CrossWlf` + `CrossWlfCoeffs`（n/tauStar/D1/D2/D3/A1/A2 + etaMin/etaMax/
  gammaDotMin 防护默认 5 / 1e6 / 1e-6）；
- `constant/phaseProperties`：`phases (melt air)` + sigma 0.025（静态）；
- `constant/physicalProperties.melt`：`heRhoThermo/pureMixture/const/
hMelt/Tait/specie/sensibleInternalEnergy`；Tait 键为
  `b1m b2m b1s b2s b3 b4 b5 b6 C smoothBand`（注意：contract 的 b4 是
  单域，Kairos Tait 模型是 b4m/b4s → 取 b4m；b6/C/smoothBand 用
  contract 默认 1.543e-07 / 0.0894 / 0.5）；`latentHeat` 必须 0
  （非零会使过渡带 Cv 为负发散，README §6）；
- `constant/physicalProperties.air`：perfectGas 静态（molWeight 28.9,
  Cp 1007, mu 1.84e-05, Pr 0.7）；
- `constant/g`（静态）、`constant/fvModels`（空）；
- `0/`：alpha.melt（inlet fixedValue 1 / vent inletOutlet / walls
  zeroGradient）、U（inlet `moldingInletVelocity` + volumetricFlowRate、
  vent pressureInletOutletVelocity、walls noSlip）、p（全 calculated）、
  p_rgh（inlet `moldingPrghPressure`、vent prghTotalPressure、walls
  fixedFluxPressure）、T（inlet fixedValue 熔温、vent inletOutlet、
  walls fixedValue 模温）；
- `system/fvSchemes`（含 div(alphaRhoPhi,e)/(alphaRhoPhi,T) 等）、
  `fvSolution`（alpha.melt.* nSubCycles 6 等）、`decomposeParDict`
  （scotch + numberOfSubdomains）均为静态模板，照抄契约。

### Kairos 数据映射

| 契约字段                         | Kairos 来源                                                                                                                     |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| injection.meltTemperature        | `process.melt_temp_c + 273.15`                                                                                                  |
| packing.switchFraction           | `process.vp_switch_volume_percent / 100`                                                                                        |
| packing.pressure                 | `(0, 1e5)` + `packing_pressure_mpa_curve` 各点 ×1e6                                                                             |
| cooling.ejectionTemperature      | `process.ejection_temp_c + 273.15`                                                                                              |
| cooling.releasePressure          | 1e5                                                                                                                             |
| momentumTransport CrossWlfCoeffs | `material.rheology` 七参 + 防护默认                                                                                             |
| physicalProperties.melt EOS      | `material.pvt`（b4 ← b4m）                                                                                                      |
| transport mu/Pr、Cp              | mu 参考值 100（CrossWlf 接管剪切黏度）；Cp ← `specific_heat` 表在熔温插值（`PropertyTable = Vec<(K, 值)>`，线性插值、端点截断） |
| 0/T 各值                         | 熔温 / 模温（`process.mold_temp_c + 273.15`）/ 300 K                                                                            |
| U volumetricFlowRate             | `mesh_volume(mesh) / process.injection_time_s`（`mesh_volume`：四面体有向体积求和）                                             |

### 边界分类（v1 启发式）

`write_poly_mesh` 的边界面按包围盒 z 分带：底 5% → `inlet`（浇口在底）、
顶 5% → `vent`（排气在顶）、其余 → `walls`；重排为「内部面 → inlet →
vent → walls」连续区段后按 startFace/nFaces 写 boundary 文件。轴向模具
假设；浇口几何（mold study 的 gate 位置）落地后替换为精确分类。

### 注意

- `format!` 模板里 OpenFOAM 花括号极多，统一用 `%KEY%` 占位符 + `replace`
  避免 `{{ }}` 转义地狱；
- `expected_fields` 更新为真实场名 `alpha.melt / p / p_rgh / T / U`
  （results 扫描消费）；
- 契约 controlDict 的验收函数对象（inletMassFlow 等）不进 Kairos 生成的
  case（那是 contract case 自验收用的）。

## Part 2 · bundle 进 VM + 作业在 VM 内执行（src-tauri）

设计（已定）：

1. `vm_deploy_bundle(app, progress)` 命令：读 manifest 中 moldingfoam 的
   归档路径 → `multipass transfer <归档> kairos:/home/ubuntu/` →
   `multipass exec kairos -- bash -lc 'mkdir -p ~/moldingfoam-env && tar
-xJf ~/bundle.tar.xz -C ~/moldingfoam-env'` → exec 测试
   `test -f ~/moldingfoam-env/openfoam14/etc/bashrc` 确认；WSL 用
   `/mnt/<盘>` 路径直接 cp。面板加「部署到虚拟机」按钮（bundle 已下载且
   VM 运行时可用）；
2. 作业执行（jobs.rs `spawn_run_script`）增加 VM 模式：VM 运行中时改为
   `multipass exec kairos -- bash -lc 'source
~/moldingfoam-env/openfoam14/etc/bashrc && cd <vm_case_dir> &&
decomposePar -force && foamRun -parallel'`；case 目录经
   `multipass mount <host_case_dir> kairos:/home/ubuntu/cases/<名>` 进 VM
   （提交前挂载；路径含空格用单引号转义，模式同现有 safe_dir）；日志
   `Time = ` 行解析不变；
3. WSL 的部署路径换算：Kairos 下载目录在 Windows 盘，WSL 内位于
   `/mnt/<盘符>/...`（`wsl -d Ubuntu-24.04 cp ...`）。

## 验收标准

- `bun run verify` 全绿；openfoam.rs 模板与契约逐文件比对通过；
- VM 内跑通 Kairos 生成 case 的填充分析（T29 合并执行）。
