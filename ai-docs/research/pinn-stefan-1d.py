"""T24 PINN spike：一维单相 Stefan 问题（熔融前沿）的物理信息神经网络解。

问题设置（无量纲，α = 1，Ste = 1）：
    液相区 0 <= x <= s(t)：u_t = u_xx
    边界：u(0, t) = 1（壁面过热），u(s(t), t) = 0（熔点）
    Stefan 条件：ds/dt = -u_x(s(t), t)
    初始：s(0) = 0，u(x, 0) = 0
相似解：s*(t) = 2·λ·sqrt(t)，u*(x, t) = 1 - erf(x / (2·sqrt(t))) / erf(λ)，
其中 λ·exp(λ²)·erf(λ) = 1/√π。

网络：u 由 (x, t) 的 MLP 表示；s(t) = t · softplus(s_net(t))（精确满足 s(0)=0）。
损失 = PDE 残差 + 壁面/界面/初始条件 + Stefan 条件（autograd 求 ds/dt）。
输出：s(t) 与 u(x,t) 相对相似解的最大误差（验收指标）。
"""

import math

import torch
import torch.nn as nn

torch.manual_seed(42)

T_MAX = 1.0
X_MAX = 2.0  # s*(1) ≈ 1.24，留余量
T_MIN_EVAL = 0.05  # 避开 t→0 的奇点区间做评估
STEPS = 9000
N_PDE = 512
N_BC = 128


def lam_root() -> float:
    """二分求 λ：λ·exp(λ²)·erf(λ) = 1/√π。"""
    lo, hi = 0.3, 1.0
    target = 1.0 / math.sqrt(math.pi)
    for _ in range(80):
        mid = (lo + hi) / 2
        if mid * math.exp(mid * mid) * math.erf(mid) < target:
            lo = mid
        else:
            hi = mid
    return (lo + hi) / 2


LAM = lam_root()


def s_exact(t: torch.Tensor) -> torch.Tensor:
    return 2 * LAM * torch.sqrt(t)


def u_exact(x: torch.Tensor, t: torch.Tensor) -> torch.Tensor:
    return 1.0 - torch.erf(x / (2 * torch.sqrt(t))) / math.erf(LAM)


class UNet(nn.Module):
    def __init__(self) -> None:
        super().__init__()
        self.net = nn.Sequential(
            nn.Linear(2, 32), nn.Tanh(),
            nn.Linear(32, 32), nn.Tanh(),
            nn.Linear(32, 32), nn.Tanh(),
            nn.Linear(32, 1),
        )

    def forward(self, x: torch.Tensor, t: torch.Tensor) -> torch.Tensor:
        return self.net(torch.cat([x, t], dim=1))


class SNet(nn.Module):
    def __init__(self) -> None:
        super().__init__()
        self.net = nn.Sequential(nn.Linear(1, 16), nn.Tanh(), nn.Linear(16, 16), nn.Tanh(), nn.Linear(16, 1))

    def forward(self, t: torch.Tensor) -> torch.Tensor:
        # s(0) = 0 由因子 t 精确满足；softplus 保证增长为正。
        return t * nn.functional.softplus(self.net(t))


u_net, s_net = UNet(), SNet()
params = list(u_net.parameters()) + list(s_net.parameters())
optimizer = torch.optim.Adam(params, lr=2e-3)
mse = nn.MSELoss()

print(f"λ = {LAM:.6f}（解析前沿系数），训练 {STEPS} 步（CPU）")

for step in range(1, STEPS + 1):
    optimizer.zero_grad()

    # PDE 配点：撒在 [0, X_MAX]×[0, T_MAX]，拒绝采样保留 x < s(t) 的液相点
    t_f = torch.rand(N_PDE, 1) * T_MAX
    x_f = torch.rand(N_PDE, 1) * X_MAX
    s_f = s_net(t_f).detach()
    mask = (x_f < s_f).squeeze()
    x_f, t_f = x_f[mask], t_f[mask]
    if x_f.numel() < 32:
        continue
    x_f.requires_grad_(True)
    t_f.requires_grad_(True)
    u_f = u_net(x_f, t_f)
    u_t = torch.autograd.grad(u_f, t_f, torch.ones_like(u_f), create_graph=True)[0]
    u_x = torch.autograd.grad(u_f, x_f, torch.ones_like(u_f), create_graph=True)[0]
    u_xx = torch.autograd.grad(u_x, x_f, torch.ones_like(u_x), create_graph=True)[0]
    loss_pde = mse(u_t, u_xx)

    # 壁面 u(0,t)=1
    t_w = torch.rand(N_BC, 1) * T_MAX
    loss_wall = mse(u_net(torch.zeros_like(t_w), t_w), torch.ones_like(t_w))

    # 界面 u(s(t),t)=0 与 Stefan 条件 ds/dt + u_x(s,t) = 0
    t_s = torch.rand(N_BC, 1) * T_MAX
    t_s.requires_grad_(True)
    s_of_t = s_net(t_s)
    u_s = u_net(s_of_t, t_s)
    loss_phase = mse(u_s, torch.zeros_like(u_s))
    ds_dt = torch.autograd.grad(s_of_t, t_s, torch.ones_like(s_of_t), create_graph=True)[0]
    s_x = s_of_t.detach().requires_grad_(True)
    u_x_at_s = torch.autograd.grad(
        u_net(s_x, t_s.detach()), s_x, torch.ones_like(s_x), create_graph=True
    )[0]
    loss_stefan = mse(ds_dt + u_x_at_s, torch.zeros_like(ds_dt))

    # 初始条件：u(x,0)=0，s(0)=0（后者由参数化精确满足）
    x_i = torch.rand(N_BC, 1) * X_MAX
    loss_ic = mse(u_net(x_i, torch.zeros_like(x_i)), torch.zeros_like(x_i))

    loss = loss_pde + 10 * loss_wall + 10 * loss_phase + 10 * loss_stefan + 5 * loss_ic
    loss.backward()
    optimizer.step()

    if step % 1500 == 0:
        print(
            f"step {step:5d}  total {loss.item():.3e}  pde {loss_pde.item():.3e}  "
            f"stefan {loss_stefan.item():.3e}  phase {loss_phase.item():.3e}"
        )

# ---------- 评估：与相似解对比 ----------
with torch.no_grad():
    t_grid = torch.linspace(T_MIN_EVAL, T_MAX, 20).reshape(-1, 1)
    s_pred = s_net(t_grid).flatten()
    s_ref = s_exact(t_grid).flatten()
    s_rel_err = ((s_pred - s_ref).abs() / s_ref).max().item()

    x_e = torch.rand(4000, 1) * X_MAX
    t_e = torch.rand(4000, 1) * T_MAX
    keep = (x_e < s_net(t_e)).squeeze() & (t_e.squeeze() >= T_MIN_EVAL)
    x_e, t_e = x_e[keep], t_e[keep]
    u_err = (u_net(x_e, t_e) - u_exact(x_e, t_e)).abs().max().item()

print(f"RESULT s_max_rel_err={s_rel_err:.4f} u_max_abs_err={u_err:.4f} lambda={LAM:.6f}")
