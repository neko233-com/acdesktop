# Changelog

## [Unreleased]

## [2.0.0] - 2026-11-12

### Changes

- 通过原生归档分发 CLI 和 C 库，外部 agent 自行选择调用方式。
- 并行 batch 根据已验证的进程身份安排语义操作，全局输入按原顺序执行。
- 修复版本检查、安装失败处理及私有状态目录相关测试。
- 配置和更新缓存复用私有文件保护，拒绝通过链接重定向写入。
- `open-system-surface` 支持自定义操作期限，并适配 Windows 11 通知中心快捷键。

### CI

- 全面修复 CI：macOS linker 符号命名对齐（Objective-C bridge 与 Rust extern "C" 声明统一 `acdesktop_` 前缀）
- Linux 容器 job 统一执行脚本的 execute 权限修复（git clone 不保留 +x 位）
- FFI header 提交文件名对齐（`agent_desktop.h` → `acdesktop.h`），本地 `cbindgen --verify` 通过
- GitHub Release 等待所有构建和 FFI 校验通过，并核对完整资产及校验和

## [1.2.0] - 2026-11-05

### Fork & Rename

- 从 agent-desktop fork 为 acdesktop（Agent Control Desktop）
- 全仓库重命名：`agent-desktop` → `acdesktop`（crate、二进制、CLI、文档）
- Objective-C bridge 导出符号统一为 `acdesktop_*` 前缀

### New Features

- **同时操作多个 batch**：`--parallel` 标志跨 bucket 并行，Observation 阶段全并行，Action 阶段同 app 串行，Global 桶最后串行
- **极快模式**：`--fast` 跳过 permission 检查、cursor overlay、视觉反馈，直出最小 JSON
- **原生自动更新**：`acdesktop update` 检查并应用新版本，`curl -fsSL | bash` 原子替换 + SHA256 校验
- **用户配置**：`~/.acdesktop/config.json` 持久化 auto_update / fast_mode / check_interval_secs
- **快捷键紧急停止**：`Ctrl+Shift+P` 全局 kill switch
- **多屏幕支持**：`--screen <id>` 指定目标显示器

### Build & CI

- workspace edition 2024，rust-version 1.89
- release.yml 支持 `v*` tag push 自动触发
- 6 平台 build matrix（macOS aarch64/x86_64、Linux x86_64/aarch64、Windows x86_64/ARM64）
- FFI cdylib + C header 自动生成（cbindgen 0.29.4）
