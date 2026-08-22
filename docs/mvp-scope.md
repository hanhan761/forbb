# 桌面端面试辅助 MVP

## 目标

作为腾讯会议、Teams 等桌面会议软件的伴随工具，在本机完成音频转写、译文显示和知识库检索。工具只显示悬浮信息，不控制会议窗口、不向会议发送内容、不自动代替用户发言。

## MVP-0.1 范围

1. Windows 系统输出回环采集，默认关注“对方”音频；默认 STT 使用本地 Whisper.cpp。
2. 实时显示原文转写；翻译作为独立流显示。
3. 支持导入 Markdown、TXT、PDF、DOCX，以及直接导入 Obsidian Vault。
4. 使用 SQLite/FTS 检索题库、岗位描述、简历和术语表。
5. 悬浮窗支持置顶、拖动、折叠、透明度和全局快捷键。
6. 用户主动点击后才查询知识库或请求辅助提示。

Whisper.cpp 的 Tiny/Base 模型通过首次运行向导或 `Settings → STT → Local Models`
下载；如果尚未下载模型，也可以把 Them 的 STT 切换为已配置的云端 provider。

Obsidian Vault 通过 `Meeting Context → Obsidian Vault → Choose Vault` 手动导入；
程序直接读取 Vault 内的 Markdown，不需要 Obsidian 插件或常驻服务，并跳过 `.obsidian`
等隐藏配置目录。若要零 Ollama 依赖地测试检索，可在 `Settings → Context Strategy`
选择 `Local RAG → Fastest`，使用 SQLite FTS5 关键词检索；混合语义检索再配置 Ollama。

## 暂不做

- 注入腾讯会议或其他会议软件的页面/进程。
- 自动发言、自动输入答案、音频回放或变声。
- 未经授权的隐藏采集、绕过平台提示或检测。
- 第一阶段的按进程音频隔离；先验证设备级 WASAPI loopback。

## 技术分层

```text
Windows WASAPI loopback
        ↓
Tauri/Rust 音频管线与 VAD
        ↓
STT provider（现有 whisper-rs / ONNX / 云端接口）
        ↓
TranscriptStore ───────────────→ Overlay transcript
        ↓
Translation provider ──────────→ Overlay translation
        ↓（用户主动触发）
SQLite FTS / RAG ───────────────→ Context / hint panel
```

## 下一条实现切口

- 将默认音频源固定为系统回环，并在设置页显式显示当前设备与音量状态。
- 为转写事件统一 `source / speaker / final / language / timestamp` 字段。
- 将翻译和知识库提示做成非阻塞的异步消费者，避免查询延迟影响原文字幕。
- 后续再接 Windows 10 2004+ 的进程级 loopback，只捕获指定会议进程。

## Windows 本地启动

在项目根目录执行：

```powershell
npm install
npx tauri dev
```

首次进行 Rust/Tauri 构建需要安装 LLVM，并在当前 PowerShell 会话设置：

```powershell
$env:LIBCLANG_PATH = 'C:\Program Files\LLVM\bin'
```

本地只验证可运行程序时，可以跳过打包和签名：

```powershell
npx tauri build --no-sign --no-bundle
```

仓库当前预置了更新公钥；正式生成带更新能力的发布包时，还需要配置对应的
`TAURI_SIGNING_PRIVATE_KEY`。本地安装包已经生成在
`src-tauri/target/release/bundle/nsis/`，无需私钥即可用于本机安装测试。
