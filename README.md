# Agent Control Desktop

> **acdesktop** — Give your AI agent reliable desktop control through the OS accessibility tree.

Agent Control Desktop (acdesktop) lets an AI agent see and operate desktop apps on macOS and Windows — without screenshots, pixel matching, or coordinate guessing. It reads the native accessibility tree, returns it as JSON with a stable ref for every actionable element, and lets the agent act on refs to click, type, drag, scroll, or read values. The observation–action loop lives in your agent; acdesktop ships no model.

It compiles to a single static CLI and a C shared library. Zero runtime dependencies, zero network calls at runtime.

---

## Install （macOS arm64 / Windows arm64）

**一键脚本 —— 最快、最轻量，零 Node 依赖：**

```bash
# macOS arm64
curl -fsSL https://raw.githubusercontent.com/neko233-com/acdesktop/main/scripts/install.sh | bash
```

```powershell
# Windows arm64 (PowerShell)
irm https://raw.githubusercontent.com/neko233-com/acdesktop/main/scripts/install.ps1 | iex
```

脚本做的事情：
1. 从 GitHub Release 下载对应平台的原生二进制
2. 用 `checksums.txt` 校验 SHA256
3. 原子替换到 `~/.local/bin`（macOS）或 `$HOME\.local\bin`（Windows）
4. 自动写入用户 PATH，刷新当前会话

其他方式：

```bash
brew install acdesktop            # macOS / Linux
# 或手动下载 release archive: https://github.com/neko233-com/acdesktop/releases
```

验证：

```bash
acdesktop status                   # platform, permissions, supported surfaces
acdesktop permissions --guide      # macOS: 引导你授予辅助功能权限
```

---

## 快速上手

```bash
# 1. 观察 —— 拿到 app 的可交互元素
acdesktop snapshot --app Finder -i
# → { "version": "2.6", "ok": true, "data": { "snapshot_id": "s8f3k2p9", "elements": [{ "ref": "@s8f3k2p9:e8", "role": "Button", "label": "新建文件夹" }] } }

# 2. 行动 —— 对某个 ref 操作
acdesktop click @s8f3k2p9:e8

# 3. 再观察 —— 确认结果
acdesktop snapshot --app Finder -i
```

每个命令返回一个 JSON envelope：

```json
{
  "version": "2.6",
  "ok": true,
  "command": "click",
  "data": { "ref": "@s8f3k2p9:e8" }
}
```

---

## 命令总览

### 观察 (observation)

| 命令 | 说明 |
|---|---|
| `acdesktop snapshot --app <name>` | 抓指定 app 的可访问性树 |
| `acdesktop snapshot --surface dock` | macOS Dock / 菜单栏等系统表面 |
| `acdesktop screenshot --app <name>` | 截屏并高亮快照元素（debug 用） |

### 行动 (action)

| 命令 | 说明 |
|---|---|
| `acdesktop click <ref>` | 左键点击 |
| `acdesktop right-click <ref>` | 右键点击 |
| `acdesktop double-click <ref>` | 双击 |
| `acdesktop type <ref> "hello"` | 在 ref 处输入文本 |
| `acdesktop set-value <ref> "..."` | 设置值（输入框、滑块等） |
| `acdesktop scroll <ref> --direction down` | 滚动 |
| `acdesktop drag <ref> --to <ref>` | 拖拽 |

### 系统 (system)

| 命令 | 说明 |
|---|---|
| `acdesktop launch --app <name>` | 启动 app |
| `acdesktop clipboard-get` | 读剪贴板 |
| `acdesktop clipboard-set "..."` | 写剪贴板 |
| `acdesktop open-system-surface <dock|menu-bar-extras|notification-center|control-center>` | 唤醒系统表面 |
| `acdesktop cursor-overlay` | 显示/隐藏光标叠加层（debug） |

### 沙箱 (sandbox)

在 macOS 和 Windows 上，可以用安全边界保护 GUI 操作，避免 agent 越过约定的窗口或 app。

---

## 同时操作多个

用 `batch --parallel` 跨 app 并行执行 —— 相当于同时操控多个。

```bash
# 并行抓取多个 app 的 UI 树（全 observation，天然零副作用）
acdesktop batch --parallel '[
  {"command":"snapshot","args":{"app":"Finder","compact":true}},
  {"command":"snapshot","args":{"app":"Chrome","compact":true}},
  {"command":"snapshot","args":{"app":"微信","compact":true}},
  {"command":"list-windows","args":{"app":"Finder"}}
]'
# 3 个 snapshot + list-windows 并行完成，耗时 ≈ 单次 snapshot
```

**安全分桶策略 —— 不牺牲正确性：**

| 命令类 | 跨 app | 同 app | 原因 |
|---|---|---|---|
| Observation（snapshot / find / list-*） | ✅ 并行 | ✅ 并行 | 纯读，零副作用 |
| Action（click / type / drag / launch） | ✅ 并行 | ❌ 严格串行 | 跨进程 UI 树互不干扰，但同 app 内焦点/状态/文本输入会竞态 |
| Global（press / key-down / mouse-* / clipboard-*） | ❌ 等所有桶跑完才执行 | ❌ 串行 | 只有一个键盘、一个鼠标、一个剪贴板 |

**不能在 `--parallel` 里用的命令**（直接 INVALID_ARGS）：`wait` / `cursor-overlay` / `sandbox` / `batch`（嵌套）/ `session gc/end` / `trace` —— 它们依赖之前 action 的结果或涉及全局状态。

并发数由 `--max-workers` 控制（默认 = CPU 核心数，上限 8）。没有引入任何新依赖，全用 `std::thread::scope`。

---

## 极快模式

跳过所有 post-action wait，让 agent-desktop 响应最快：

```bash
# 单次
acdesktop --fast click @s8f3k2p9:e5

# 永久：写入 ~/.acdesktop/config.json
acdesktop config set fast_mode true
```

开启后所有 `--wait-for` / `--wait-for-gone` / `--wait-timeout` 参数被忽略。

---

## 自动更新 + 用户配置

acdesktop 每次调用时静默检查新版本（默认 6 小时一次，可配置）。发现新版本只提示，不自动安装。

```bash
acdesktop update check          # 手动检查
acdesktop update apply          # 调用 install.sh / install.ps1 自更新，替换后下次启动生效
acdesktop update auto on        # 开启自动更新提示（默认开启）
acdesktop update auto off       # 完全关闭
acdesktop update auto status

acdesktop config show           # 查看完整配置
acdesktop config set auto_update false
acdesktop config path           # 打印 ~/.acdesktop/config.json 路径
```

`~/.acdesktop/config.json`：

```json
{
  "auto_update": true,
  "fast_mode": false,
  "last_version_check": 0,
  "check_interval_secs": 21600
}
```

也可以直接用环境变量一键关闭自动更新（CI 脚本场景）：

```bash
ACDESKTOP_NO_AUTO_UPDATE=1 acdesktop status
```

---

## 设计原则

- **原生，不猜像素。** 所有定位都走 OS 无障碍 API（macOS Accessibility、Windows UI Automation），不做屏幕截图 + 图像匹配。
- **快速。** release profile 默认 `-C opt-level=z` + LTO + panic=abort + strip；二进制体积 < 15 MB。无网络、无 JIT、无垃圾回收。
- **稳定的 ref。** 每个元素有一个 snapshot 内唯一的 token；跨 snapshot 解析失败时，返回 `AMBIGUOUS_TARGET` 并告诉 agent 应该重新观察，而不是误点。
- **分层架构。** `crates/core` 放平台无关的通用逻辑和策略；`crates/macos`、`crates/windows`、`crates/linux` 各一个 adapter，各自负责平台特定的 API 调用。

```
 ┌──────────────┐   ┌──────────────┐   ┌──────────────┐
 │   CLI 层     │   │   FFI 层     │   │   MCP 层     │   ← 入口
 └──────┬───────┘   └──────┬───────┘   └──────┬───────┘
        └───────────┬───────┴───────────┘
                    ▼
             ┌────────────┐
             │ core crate │   ← 平台无关逻辑、策略、批量调度
             └──┬───┬───┬─┘
                │   │   │
      ┌─────────┘   │   └──────────┐
      ▼             ▼              ▼
┌──────────┐ ┌──────────┐   ┌──────────┐
│  macOS   │ │ Windows  │   │  Linux   │   ← 平台 adapter
└──────────┘ └──────────┘   └──────────┘
```

---

## 性能

| 指标 | macOS arm64 | Windows arm64 |
|---|---|---|
| 冷启动（原生二进制，无 Node） | ~12 ms | ~15 ms |
| `snapshot --app Finder -i` | ~40 ms | ~55 ms |
| `click` 单次行动 | ~8 ms | ~12 ms |
| 二进制体积 | < 12 MB | < 11 MB |

---

## 安全

快照、剪贴板、截屏、命令输出都可能包含隐私内容。请：
- 不要把它们写入公共日志或上传
- 用 `sandbox` 命令限制 agent 的操作边界
- 在沙盒中运行 agent-desktop 批量任务

编译产物已通过 Sigstore provenance 签名。下载后可验证：

```bash
gh attestation verify acdesktop-v<version>-<target>.tar.gz --repo neko233-com/acdesktop
```

---

## 构建

```bash
cargo build --release -p acdesktop
# 产物: target/release/acdesktop (macOS/Linux) 或 target/release/acdesktop.exe (Windows)

# 构建 CLI + 所有平台 adapter：
cargo build --release -p acdesktop --workspace --exclude acdesktop-linux
```

---

## 许可证

[Apache License 2.0](LICENSE)

---

## 相关链接

- GitHub Release: https://github.com/neko233-com/acdesktop/releases
- Issue Tracker: https://github.com/neko233-com/acdesktop/issues
