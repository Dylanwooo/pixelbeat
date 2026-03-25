# pixelbeat

> 为长时间编码而生的像素风终端音乐播放器守护进程。

[English](README.md) | [中文](README_CN.md)

[![Rust](https://img.shields.io/badge/Rust-2021_Edition-E3893E?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-E3893E.svg)](LICENSE)
[![macOS](https://img.shields.io/badge/macOS-supported-E3893E?logo=apple&logoColor=white)]()

```
┌ PIXELBEAT ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─┐
│ ◉ ──────────────●━━━━━━━━━━━━━━ ◎  ▶ 2:47/4:12  │
│ Sleepy Fish - A Rainy Night in Kyoto  ▅▂█▄▇▁▃▆▂▅ │
│  ⏮   ⏸   ⏭       🔁  🔀                         │
└──────────────────────────────────────────────────┘
```

pixelbeat 作为后台守护进程运行，通过轻量 CLI (`px`) 控制播放。它通过 mpv 即时串流 YouTube 播放列表，通过 rodio 播放本地音频文件，并内置了 chillhop 和 lofi 电台。动态磁带机 UI 和频谱可视化可直接嵌入 Claude Code 状态栏、tmux 或 starship。

## 功能特性

- **守护进程架构** -- 启动一次，通过 Unix socket IPC 随时随地控制
- **YouTube 串流** -- 通过 mpv 即时播放任何 YouTube 视频或播放列表（无需下载）
- **本地文件播放** -- 支持 MP3、FLAC、WAV、OGG、M4A、AAC、Opus、AIFF（rodio/symphonia）
- **内置电台** -- Chillhop 和 lofi 电台，开箱即用
- **磁带机 UI** -- 动态卷轴式磁带可视化，带播放头轨迹
- **频谱可视化** -- 32 条节拍同步频谱分析器，Anthropic 橙色渐变
- **TUI 模式** -- 基于 ratatui 的全屏终端界面
- **状态栏集成** -- 一行命令集成 Claude Code、tmux 或 starship
- **格式模板引擎** -- 用 `{tape:30}`、`{spectrum:16}`、`{cassette:50}` 等 token 组合自定义状态栏
- **随机和循环** -- 通过配置文件跨会话持久化模式切换
- **可点击控制** -- 在支持的终端中通过 OSC 8 超链接按钮控制（上一首/暂停/下一首/循环/随机）

## 快速开始

**前置依赖**：[Rust](https://rustup.rs/) 工具链、[mpv](https://mpv.io/) 和 [yt-dlp](https://github.com/yt-dlp/yt-dlp)（用于 YouTube）。

```bash
# 安装依赖（macOS）
brew install mpv yt-dlp

# 克隆并构建
git clone https://github.com/Dylanwooo/pixelbeat.git
cd pixelbeat
cargo build --release

# 添加到 PATH
cp target/release/px ~/.local/bin/  # 或 PATH 中的任何位置

# 启动守护进程并播放 YouTube
px daemon &
px yt "https://www.youtube.com/watch?v=jfKfPfyJRdk"
```

几秒钟内你就能听到音乐。运行 `px tui` 打开全屏播放器，或 `px status` 在终端查看磁带机 UI。

## 安装

### 从源码安装（推荐）

```bash
git clone https://github.com/Dylanwooo/pixelbeat.git
cd pixelbeat
cargo install --path .
```

这会将 `px` 二进制文件安装到 `~/.cargo/bin/`，确保该目录在你的 `PATH` 中。

### 依赖

| 依赖 | 是否必需 | 用途 | 安装方式 |
|-----|---------|------|---------|
| **Rust**（2021 edition） | 是 | 构建工具链 | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| **mpv** | YouTube 需要 | 音频串流后端 | `brew install mpv` |
| **yt-dlp** | YouTube 需要 | 播放列表解析和音频提取 | `brew install yt-dlp` |

本地文件播放和内置电台无需 mpv 或 yt-dlp。

## 使用方法

### 启动守护进程

```bash
# 启动守护进程（阻塞终端）
px daemon

# 启动并立即播放目录
px daemon --play ~/Music

# 后台启动
px daemon &
```

### 播放本地文件

```bash
# 播放单个文件
px play ~/Music/song.mp3

# 播放目录中所有支持的文件
px play ~/Music/chillhop/

# 恢复播放（如果暂停）
px play
```

支持格式：MP3、FLAC、WAV、OGG、M4A、AAC、Opus、AIFF。

### 播放 YouTube

通过 mpv 串流任何 YouTube 视频或播放列表，即时播放，无需下载到磁盘。

```bash
# 播放单个视频
px yt "https://www.youtube.com/watch?v=jfKfPfyJRdk"

# 播放播放列表
px yt "https://www.youtube.com/playlist?list=PLOzDu-MXXLliO9fBNZOQTBDddoA3FzZUo"
```

### 内置电台

```bash
# 列出可用电台
px radio

# 播放电台
px radio chillhop
px radio lofi
```

电台：**chillhop**（来自 Chillhop Music 的 30 首精选曲目）、**lofi**（来自 Lofi Girl 的 15 首曲目）。

### 播放控制

```bash
px toggle          # 播放/暂停切换
px pause           # 暂停
px next            # 下一首
px prev            # 上一首
px stop            # 停止播放
px vol 0.5         # 设置音量（0.0 到 1.0）
px shuffle         # 切换随机模式
px repeat          # 切换循环模式
```

### TUI 模式

打开带有频谱可视化、进度条和键盘控制的全屏终端 UI。

```bash
px tui
```

**TUI 快捷键**：

| 按键 | 操作 |
|------|------|
| `Space` | 播放/暂停 |
| `n` 或 `Right` | 下一首 |
| `p` 或 `Left` | 上一首 |
| `+` 或 `Up` | 音量增大 |
| `-` 或 `Down` | 音量减小 |
| `s` | 切换随机模式 |
| `r` | 切换循环模式 |
| `q` 或 `Esc` | 退出 TUI |

### 状态栏输出

查询当前播放器状态，支持自定义格式。

```bash
# 默认：渲染完整磁带机组件
px status

# 自定义格式字符串
px status --format "{icon} {title:.25} {bar:12} {elapsed}/{duration}"

# 仅频谱
px status --format "{spectrum:32}"

# 自定义宽度的磁带机
px status --format "{tape:40}"
```

### 停止守护进程

```bash
px quit
```

## 状态栏集成

### Claude Code

运行安装向导：

```bash
px setup claude-code
```

或手动添加到 `~/.claude/statusline.sh`：

```bash
# pixelbeat 音乐播放器状态
if command -v px &>/dev/null; then
    PX_STATUS=$(px status --format "♪ {title:.25} {icon} {bar:12} {elapsed}/{duration}" 2>/dev/null)
    if [ -n "$PX_STATUS" ]; then
        echo "$PX_STATUS"
        echo "$(px status --format "  {spectrum:32}" 2>/dev/null)"
    fi
fi
```

也可以直接使用 [`integrations/claude-code.sh`](integrations/claude-code.sh)。

### tmux

运行 `px setup tmux`，或添加到 `~/.tmux.conf`：

```tmux
set -g status-right '#(px status --format "{icon} {title:.20} {bar:8} {elapsed}" 2>/dev/null)'
set -g status-interval 1
```

然后重新加载：`tmux source-file ~/.tmux.conf`

### Starship

运行 `px setup starship`，或添加到 `~/.config/starship.toml`：

```toml
[custom.music]
command = "px status --format '{icon} {title:.15} {elapsed}'"
when = "px status"
format = "[$output]($style) "
style = "bold #E3893E"
```

## 格式模板 Token

在 `px status --format "..."` 中使用以下 token 构建自定义状态栏。

| Token | 说明 | 示例输出 |
|-------|------|---------|
| `{title}` | 曲目标题（完整） | `Sleepy Fish - A Rainy Night in Kyoto` |
| `{title:.N}` | 曲目标题，截断到 N 个字符 | `Sleepy Fish - A Rain…` |
| `{icon}` | 播放/暂停图标 | `▶` 或 `⏸` |
| `{bar:N}` | 进度条，N 字符宽 | `████░░░░░░` |
| `{tape:N}` | 磁带卷轴可视化，N 字符宽 | `◉ ──────●━━━━━━━ ◎` |
| `{spectrum:N}` | 动态频谱条，N 条宽 | `▅▂█▄▇▁▃▆▂▅▃▇▁▄▆▂` |
| `{cassette:N}` | 完整磁带机组件（多行），N 字符宽 | *（见上方示例）* |
| `{elapsed}` | 已播放时间 | `2:47` |
| `{duration}` | 总时长 | `4:12` |
| `{vol}` | 音量百分比 | `80%` |
| `{vol:bar:N}` | 音量条，N 字符宽 | `████░` |
| `{index}` | 当前曲目编号（从 1 开始） | `3` |
| `{count}` | 总曲目数 | `12` |
| `{shuffle}` | 随机模式指示器（激活时高亮） | `🔀` |
| `{repeat}` | 循环模式指示器（激活时高亮） | `🔁` |
| `{modes}` | 循环 + 随机组合指示器 | `🔁 🔀` |
| `{controls}` | 文字控制说明 | `⏮ prev ⏯ toggle ⏭ next 🔁 loop` |
| `{buttons}` | 可点击 OSC 8 超链接按钮 | `⏮  ⏸  ⏭  🔁  🔀` |

所有 token 使用 Anthropic 橙色 ANSI 颜色渲染（`#E3893E` 主色，带明亮/暗淡/背景变体）。

## 配置

pixelbeat 从 `~/.config/pixelbeat/config.toml` 读取配置。所有字段均为可选，未设置时使用合理的默认值。

```toml
# 守护进程启动时的默认源："local"、"chillhop"、"lofi"、"youtube"
source = "local"

# YouTube 播放列表 URL（当 source = "youtube" 时使用）
youtube_url = "https://www.youtube.com/watch?v=jfKfPfyJRdk"

# 本地音乐目录（当 source = "local" 时使用）
music_dir = "~/Music/pixelbeat"

# 默认音量（0.0 - 1.0）
volume = 0.8

# 自动循环
repeat = false

# 随机播放
shuffle = false
```

### 配置参考

| 键 | 类型 | 默认值 | 说明 |
|----|------|-------|------|
| `source` | `string` | *（无）* | 启动时自动播放的源。可选 `"local"`、`"chillhop"`、`"lofi"`、`"youtube"`。省略时如果 `music_dir` 存在则加载本地文件。 |
| `youtube_url` | `string` | *（无）* | YouTube 视频或播放列表 URL。仅在 `source = "youtube"` 时使用。 |
| `music_dir` | `string` | `"~/Music/pixelbeat"` | 扫描本地音频文件的目录，支持波浪号展开。 |
| `volume` | `float` | `0.8` | 初始音量级别，从 `0.0`（静音）到 `1.0`（最大）。 |
| `repeat` | `bool` | `false` | 播放列表到末尾时循环。电台模式会强制开启。 |
| `shuffle` | `bool` | `false` | 随机曲目顺序。当同时启用循环时，每次循环重新随机。 |

## 架构

```
┌─────────┐    Unix socket IPC    ┌────────────────────┐
│  px CLI  │ ──────────────────── │  px daemon         │
│          │   JSON 命令           │                    │
│  px tui  │ ◄────────────────── │  ┌──────────────┐  │
└─────────┘   JSON 响应           │  │ 播放器        │  │
                                  │  │  - rodio     │  │  本地文件
                                  │  │  - mpv IPC   │──│──────────
                                  │  └──────────────┘  │
                                  │  ┌──────────────┐  │  YouTube
                                  │  │ mpv 进程     │──│──────────
                                  │  └──────────────┘  │  (yt-dlp)
                                  │  ┌──────────────┐  │
                                  │  │ 频谱          │  │  电台串流
                                  │  │ 分析器       │──│──────────
                                  │  └──────────────┘  │  (HTTP)
                                  └────────────────────┘
```

**守护进程**（`px daemon`）-- 长驻进程，拥有音频输出。在 Unix socket `$XDG_RUNTIME_DIR/pixelbeat.sock`（回退到 `/tmp/pixelbeat.sock`）上监听。以 50ms 间隔轮询更新播放位置、检测曲目结束和生成频谱数据。

**CLI**（`px <command>`）-- 轻量客户端，将命令序列化为 JSON，通过 Unix socket 发送，并打印响应。每次调用连接、发送一条命令、读取一条响应后退出。

**TUI**（`px tui`）-- 基于 ratatui 的全屏界面，每 100ms 轮询守护进程状态并渲染实时频谱可视化。所有输入都被转换为与 CLI 相同的 IPC 命令。

**播放引擎** -- 本地文件通过 rodio（基于 symphonia）解码。YouTube 音频通过 mpv 子进程串流，使用 mpv 的 JSON IPC 协议控制。播放器根据源类型在 rodio 和 mpv 之间透明切换。

**频谱分析器** -- 以约 20 FPS 生成 32 条节拍同步动画数据。使用确定性伪随机波形，配合对比度增强和尖峰注入，实现有力的反应式视觉效果。基于 FFT 的真实 PCM 分析路径已实现，供未来使用。

## 贡献

欢迎贡献。以下是入门方式：

1. Fork 仓库并克隆你的 fork。
2. 创建功能分支：`git checkout -b my-feature`。
3. 进行修改。提交前运行 `cargo fmt` 和 `cargo clippy`。
4. 本地测试：用 `cargo run -- daemon` 启动守护进程，然后用 CLI 测试你的修改。
5. 向 `main` 分支提交 Pull Request。

如果你发现 bug 或有功能想法，请先提 issue 讨论。

## 许可证

MIT -- 详见 [LICENSE](LICENSE)。

Copyright (c) 2026 Dylan Woo
