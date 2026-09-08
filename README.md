<p align="center">
  <img src="public/nexq-icon.png" alt="NexQ" width="120">
</p>

<h1 align="center">NexQ · Qwen 纯 API 中文版</h1>

<p align="center">把会议声音变成字幕，把对方的问题变成可以直接使用的简短回答。</p>

<p align="center">

[![Release](https://img.shields.io/github/v/release/hanhan761/forbb?style=flat-square&color=blue)](https://github.com/hanhan761/forbb/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/hanhan761/forbb/release.yml?style=flat-square&label=构建)](https://github.com/hanhan761/forbb/actions/workflows/release.yml)
[![License](https://img.shields.io/badge/license-MIT-green?style=flat-square)](LICENSE)

</p>

## 这是什么

NexQ 是一个 Windows 会议助手。它在本机采集麦克风和会议输出声音，使用你配置的 Qwen API 做语音识别和回答，并把结果显示在悬浮窗里。

这个仓库只维护一个版本：**简体中文、纯 API、Qwen 专用版**。

- 不下载 Whisper、Ollama、ONNX 等本地 AI 模型。
- 不要求配置 OpenAI、Gemini、Deepgram 等其他供应商。
- AI 能力只需要一把 Qwen API Key。
- 音频采集、会议记录、知识库文件和索引保存在本机。

## 第一步：配置 Qwen API

安装并打开 NexQ 后，第一屏就是 Qwen 配置。填入 Qwen API Key，测试成功后即可使用全部 AI 功能：

| 功能 | 使用方式 |
| --- | --- |
| 实时语音识别 | Qwen3-ASR-Flash |
| 自动回答、总结、行动项 | Qwen 文本模型 |
| 翻译和语言处理 | Qwen 文本模型 |
| 知识库检索 | 本机关键词索引，不额外消耗 API |

因此，用户不需要理解“配置多个模型”或“下载模型文件”。配置好 Qwen 后，直接选择音频设备并开始会议即可。

Qwen 官方资料：[创建 API Key](https://help.aliyun.com/zh/model-studio/get-api-key)、[文本 API](https://help.aliyun.com/en/model-studio/qwen-api-via-openai-chat-completions)、[Qwen ASR](https://help.aliyun.com/en/model-studio/qwen-asr-api-reference)。

## 五分钟开始使用

1. 下载并安装 Windows 安装包。
2. 首次启动时填写 Qwen API Key，并点击测试。
3. 在音频设置中选择：
   - **我方**：你的麦克风。
   - **对方**：会议软件实际播放声音的输出设备。
4. 如果使用耳机，把耳机选在“对方 / 系统声音”里，然后播放一段会议声音确认电平条有变化。
5. 在知识库中添加常用资料、项目文档或简历，并等待索引完成。
6. 开始会议。NexQ 会自动识别字幕和问题。

## 默认工作流

```text
麦克风 / 耳机输出
        ↓
本机音频采集
        ↓
Qwen3-ASR-Flash 识别字幕
        ↓
识别对方的中文或英文问题
        ↓
本机知识库关键词检索
        ↓
Qwen 生成 15–30 秒简短回答
        ↓
悬浮窗显示回答
```

自动回答默认开启。识别到问题后，系统会固定执行“知识库优先 + Qwen 简答”，不会自动切换到网页搜索，也不会等待用户再点击一次。

如果上一条回答还在生成，新的问题会按顺序排队（最多保留 3 条），避免并发请求导致回答丢失。回答失败时，悬浮窗会显示原因，通常是 API Key、网络或知识库索引问题。

## 音频采集和耳机

### 应该怎么选

- “我方”选择麦克风。
- “对方”选择会议软件播放声音的输出设备，而不是麦克风。
- 使用蓝牙耳机时，先确认 Windows 声音设置中的会议输出设备名称，再在 NexQ 中选择同一个设备。

Windows 版本使用 WASAPI shared loopback 直接读取选中的输出端点。耳机、USB 声卡、扬声器和蓝牙输出都走同一条系统音频链路；不会因为设备打开失败而静默改抓另一个默认设备。

### 没有声音时按这个顺序检查

1. 在 Windows 音量混合器里确认会议软件确实输出到你在 NexQ 选择的设备。
2. 在 NexQ 的音频设置中刷新设备列表，重新选择耳机。
3. 播放会议声音，观察“对方 / 系统声音”的电平条是否变化。
4. 如果耳机刚刚连接、断开或切换了蓝牙模式，停止会议后刷新设备，再重新开始。
5. 检查应用日志。如果目标设备不可用，NexQ 会直接报告 loopback 初始化失败，不会伪装成“已采集”。

耳机输出能被采集，不代表能采集到耳机麦克风；耳机麦克风属于“我方”输入设备，需要单独选择。

## 知识库怎么工作

知识库文件保存在本机。添加文件后，NexQ 会切分文本并建立 SQLite FTS5 关键词索引。纯 API 版本默认使用关键词检索，不需要安装 Ollama 或下载 embedding 模型。

自动回答时，问题本身会作为第一检索词；最近一小段会议上下文会作为补充检索词。检索到的资料会和字幕一起交给 Qwen，回答会尽量基于资料，不会凭空扩展。

如果没有匹配资料，Qwen 仍会根据当前会议上下文回答；界面会标注没有找到知识库资料。重要问题请人工核对。

## 会议中可以做什么

- 实时字幕：同时显示我方和对方发言。
- 自动问题识别：支持中文疑问词、中文全角问号和英文问题句式。
- 自动简答：默认 15–30 秒，适合直接阅读或组织语言。
- 手动提问：在悬浮窗中输入问题，仍然走本地知识库优先流程。
- 会议总结：整理主题、决定、行动项和未解决问题。
- 翻译：使用 Qwen 完成语言转换。
- 录音与历史：在本机保存会议记录，便于会后复盘。
- 音频恢复：耳机、USB 或蓝牙端点短暂断开后，应用会尝试重新连接；持续不可用时会显示明确错误。

## 安装、升级与卸载

当前安装包：[NexQ_2.20.14_x64-setup.exe](https://github.com/hanhan761/forbb/releases/download/v2.20.14/NexQ_2.20.14_x64-setup.exe)

也可以打开 [GitHub Releases](https://github.com/hanhan761/forbb/releases)，下载最新的 `x64-setup.exe`。

- 安装器是 Windows 当前用户安装，不需要管理员权限。
- 普通用户不需要 Node.js、Rust、Python 或本地 AI 模型。
- 升级时直接运行新的安装包即可，不需要先卸载旧版本。
- 一般情况下，Qwen Key、应用设置和会议历史会保留在本机。
- 卸载应用不会替你删除会议记录；如需彻底清理，请先在应用内导出或删除记录，再清理应用数据目录。

## 隐私和费用

音频采集、录音、会议历史和关键词索引默认留在本机。只有在调用 Qwen 语音识别、文本回答或翻译时，相关音频/文字才会发送到阿里云 DashScope。API 调用费用、限流和数据保留以 Qwen 官方规则为准。

请在录音或使用 AI 辅助前遵守会议平台、公司、学校及所在地的隐私和录音规定。

## 从源码构建

```bash
npm ci
npm run tauri:build
```

默认构建就是本仓库维护的纯 API 中文版，不会因为构建参数而下载本地 AI 模型。

## 发布

发布流程由 [.github/workflows/release.yml](.github/workflows/release.yml) 完成。推送 `v*` 标签后，GitHub Actions 会构建并发布 Windows `x64-setup.exe`。

## 许可证

[MIT License](LICENSE)
