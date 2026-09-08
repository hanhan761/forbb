<p align="center">
  <img src="public/nexq-icon.png" alt="NexQ" width="120">
</p>

<h1 align="center">NexQ 纯 API 中文版</h1>

<p align="center">
  Windows AI 会议助手：本地采集音频和保存会议记录，AI 能力通过云端 API 完成。
</p>

<p align="center">

[![Release](https://img.shields.io/github/v/release/hanhan761/forbb?style=flat-square&color=blue)](https://github.com/hanhan761/forbb/releases)
[![Build](https://img.shields.io/github/actions/workflow/status/hanhan761/forbb/release.yml?style=flat-square&label=构建)](https://github.com/hanhan761/forbb/actions/workflows/release.yml)
[![License](https://img.shields.io/badge/license-MIT-green?style=flat-square)](LICENSE)

</p>

## 版本定位

这个仓库只维护一个版本：**简体中文、纯 API、Windows 安装版**。

- 不包含本地 Whisper、Ollama、ONNX 或其他本地 AI 模型。
- 语音识别、翻译和大语言模型使用你配置的云端 API。
- 麦克风/系统声音采集、录音、会议历史和 SQLite 关键词检索保留在本地。
- 应用界面、设置、弹窗、托盘、悬浮窗和 Windows 安装器均为简体中文。

## 下载安装

当前版本：[NexQ_2.20.12_x64-setup.exe](https://github.com/hanhan761/forbb/releases/download/v2.20.12/NexQ_2.20.12_x64-setup.exe)

也可以打开 [GitHub Releases](https://github.com/hanhan761/forbb/releases)，下载名称以 **NexQ 纯 API 中文版** 开头的最新 `x64-setup.exe`。

安装器使用当前用户模式，不需要管理员权限，也不需要安装 Node.js、Rust 或本地模型。

升级时直接下载新安装包并运行即可，不需要先卸载旧版本；应用设置和会议历史通常会保留。

## 配置 API

安装后打开“设置”，按需配置：

- **语音识别：** Deepgram、OpenAI Whisper、Azure Speech、Groq Whisper
- **大语言模型：** OpenAI、Qwen、Anthropic、Groq、Gemini、OpenRouter 或自定义 API
- **翻译：** Microsoft、Google、DeepL 或远程 LLM

API 密钥通过应用的凭据管理器保存，不写入仓库，也不会硬编码到程序中。

## 从源码构建

```bash
npm ci
npm run tauri:build
```

默认构建就是纯 API 中文版。普通用户不需要执行源码构建，直接下载安装包即可。

## 发布

发布流程只有一个：[纯 API 中文版发布工作流](.github/workflows/release.yml)。

```bash
git tag v2.20.12
git push origin v2.20.12
```

推送 `v*` 标签后，GitHub Actions 会构建并发布 Windows `x64-setup.exe` 安装包。

## 隐私说明

音频、录音、会议历史和本地索引默认保存在设备上。只有在调用语音识别、翻译或 LLM 时，相关内容才会发送到你配置的云端服务。请根据会议平台、学校或当地法规处理录音和 AI 辅助事宜。

## 许可证

[MIT License](LICENSE)
