# Findings & Decisions

## Requirements

### 1. 问答触发与翻译逻辑

- 修复用户提问时只翻译、不回答的问题。
- 排查当前似乎需要按空格才能触发回答的原因。
- 排查自动翻译偶发的内容错配问题。
- 重点检查语音分段、上下文、模式判断和回答触发逻辑。

### 2. 回答语言可选

- 设置中增加：中文、English、中英双语。
- 记忆用户上次选择。
- 双语统一为中文在前、英文在后。

### 3. 悬浮窗透明度

- 增加滑块调节。
- 置顶悬浮窗运行中实时生效。
- 记忆上次透明度。

### 4. 知识库文件夹管理

- 支持批量删除文件。
- 支持一键删除整个知识库文件夹。
- 支持更换/重新选择知识库目录。
- 支持清空旧知识库后重新导入。

### 范围外

- 耳机/麦克风/输入设备问题已确认是用户自身设置，不改软件。

## Research Findings

- 当前工作目录已克隆 `https://github.com/hanhan761/forbb`，分支为 `main`，提交为 `d00c883`（v2.20.14）。
- 仓库 README 明确将当前产品定义为“简体中文、纯 API、Qwen 专用版”。
- `src/lib/buildMode.ts` 将 `REMOTE_ONLY` 固定为 `true`；当前远程 provider 列表为 Qwen ASR、Qwen LLM 和 LLM 翻译。
- `src-tauri/Cargo.toml` 默认启用 `remote-only`；`local-ai` 仅作为可选兼容特性，不属于默认构建。
- 纯 API 版本仍保留 Tauri 桌面 UI、音频采集和本地 SQLite 会议/知识库数据；“纯 API”指 AI 服务走云端 API，不等于无 UI 的 HTTP 服务。
- `npm ci` 已完成；`npm run build` 通过，只有既有分包和 Browserslist 警告。
- `cargo check --locked --manifest-path src-tauri/Cargo.toml` 通过。
- 构建后 Git 工作树保持干净，没有源码改动。

## Candidate Code Areas to Inspect

- 问答/翻译/语音链路：`src/hooks/useSpeechRecognition.ts`、`src/hooks/useTranscript.ts`、`src/hooks/useTranslation.ts`、`src/hooks/useSummaryGeneration.ts`、`src/overlay/QuestionDetector.tsx`、`src-tauri/src/intelligence/`、`src-tauri/src/commands/intelligence_commands.rs`。
- Qwen API 和流式回答：`src-tauri/src/llm/`、`src-tauri/src/stt/qwen_asr.rs`、`src-tauri/src/translation/`、`src/lib/ipc.ts`、`src/lib/events.ts`。
- 设置及持久化：`src/stores/configStore.ts`、`src/lib/types.ts`、`src/settings/QwenSettings.tsx`、`src/settings/GeneralSettings.tsx`、`src/settings/SettingsOverlay.tsx`。
- 悬浮窗/窗口控制：`src/App.tsx`、`src/overlay/OverlayView.tsx`、`src/lib/windows.ts`、`src-tauri/src/lib.rs`、`src-tauri/tauri.conf.json` 和 capabilities。
- 知识库 UI 与状态：`src/context/FileUpload.tsx`、`src/context/ContextPanel.tsx`、`src/context/ResourceCard.tsx`、`src/context/RagIndexBar.tsx`、`src/stores/ragStore.ts`。
- 知识库 IPC/数据库：`src/lib/ipc.ts`、`src-tauri/src/commands/rag_commands.rs`、`src-tauri/src/commands/context_commands.rs`、`src-tauri/src/db/rag.rs`、`src-tauri/src/rag/`。

## Current Code-Path Map (initial)

- 前端 IPC 层由 `src/lib/ipc.ts` 暴露 `generateAssist`、`pushTranscript`、`translateText`、`translateSegments` 和知识库相关调用。
- 后端 `src-tauri/src/commands/intelligence_commands.rs` 负责组装自定义问题/自动检测问题、前端最终语音段、路由和流式回答。
- 后端 `src-tauri/src/intelligence/question_detector.rs` 已有中文无问号检测测试，但需要确认检测事件是否一定进入回答调度。
- `src/lib/chineseLocale.tsx` 暴露了“Auto + Space”和“Space to ask”等现有交互文案，提示空格可能只是手动提问入口，而非自动回答触发本身。
- 自动翻译入口分布在 `src/hooks/useTranslation.ts`、`src/hooks/useSpeechRecognition.ts`/`useTranscript.ts` 及 `src-tauri/src/commands/translation_commands.rs`，需要核对 segment id、说话方和目标语言是否在异步回调中错位。
- 配置和持久化主要集中在 `src/stores/configStore.ts`、`src/lib/types.ts` 及各设置面板；新回答语言和透明度应接入同一持久化生命周期。
- 知识库当前由 `src/context/*`、`src/stores/ragStore.ts`、`src-tauri/src/commands/rag_commands.rs` 和 `src-tauri/src/db/rag.rs` 共同维护，删除/切换必须同时处理磁盘路径和 SQLite 索引。

## Concrete Discoveries

- `QuestionDetector` 当前仅在 `OverlayView` 中渲染，并且 `OverlayView` 只有在 AI Actions 的 `autoTrigger` 为真时才渲染问题区域；关闭该配置时，空格快捷键仍会调用 `generateAssist("Assist")`，因此用户会感知为“必须按空格才能回答”。
- `QuestionDetector` 对 `question_detected` 事件和 overlay 本地 transcript store 都做检测；launcher 的 `push_transcript` 与 overlay 的跨窗口 transcript 事件是两条不同的传输路径，事件到达顺序需要固定。
- `generateAssist` 的 IPC Promise 会等待后端 `generate_assist` 完整返回；后端在 `IntelligenceEngine::generate_assist` 内等待 Qwen 流结束后才清除 generating 状态，因此自动回答队列可以按完整请求生命周期管理。
- `get_response_language_instruction()` 目前在 `intelligence_commands.rs` 中硬编码为 Simplified Chinese，且 `prompt_templates.rs` 的默认模板也包含中文指令；新增语言必须覆盖动态指令并兼容自定义 action prompt。
- `ConfigState` 已有 `overlayOpacity`、setter、plugin-store 读取和跨窗口 key change；`OverlayView` 只将其用于 DOM 背景 alpha，尚未调用 Tauri `WebviewWindow` 的原生透明度 API。
- `GeneralSettings.tsx` 已有透明度滑块（0.1–1.0、0.01 步进），所以该需求的缺口主要是原生窗口背景/实时同步，而不是重新设计存储。
- `ContextManager::load_file()` 会把选中的文件复制到应用 context 目录，再将复制路径写入 `context_resources`；`remove_file()` 删除的是该复制文件。Obsidian Vault 目前只是把 Markdown 文件逐个导入，未持久化 vault/folder 分组。
- 当前知识库删除是单文件确认：`ContextPanel` 调用 `removeContextFile` 后再异步调用 `removeFileRagIndex`；后端删除操作忽略了部分 DB 清理错误，且 UI 不提供多选、分组或切换入口。
- RAG SQLite 通过 `rag_chunks` 与 FTS5 触发器关联，删除 folder 时必须先删除对应 file ids 的 embeddings/chunks，再删除 context resources，并让内存 ContextManager 同步移除。

## Debug Feedback Loop Plan

- 首先使用现有 Rust 单元测试覆盖问题检测、上下文拼接和语言提示词构造；如果现有 seam 不能复现“只翻译不回答”，新增最小的纯函数 seam 测试，而不是依赖真实 Qwen 网络。
- 对前端异步链路使用确定性 mock/fixture，验证一个最终问题段是否同时保留在翻译任务和回答任务中，且回答调用不依赖键盘空格事件。
- 对知识库使用临时目录 + SQLite 测试数据库，验证批量/文件夹删除后的磁盘、索引、状态三者一致。
- 真实 Qwen API 不作为默认回归依赖；网络 API 只用于人工验收或可选冒烟测试。

## Ranked Bug Hypotheses

1. `QuestionDetector` only exists in the overlay and gates automatic generation on the persisted `autoTrigger` flag; when that flag is false, translation continues while answers require the `Space` → `Assist` shortcut.
2. `QuestionDetector`'s `autoAssistInFlightRef`, React `isStreaming` state, and `generateAssist` IPC completion may have an async race that releases the in-flight guard before the actual stream ends, causing dropped or re-entrant automatic answers.
3. Backend `push_transcript` and frontend `transcript_final` are separate event paths; source/speaker filtering or listener timing can allow a segment to reach subtitles/translation while missing question detection.
4. Automatic translation may misassociate results when several final segments, debounced timers, or target-language changes overlap; the segment id and original text must be validated at the result boundary.
5. Answer generation may fall back to the engine's most recent detected question instead of the just-triggered segment when the custom question is absent or event ordering changes.

Predictions and probes are recorded here before implementation so each fix can be tied to a falsifiable behavior.

## Technical Decisions

| Decision | Rationale |
|----------|-----------|
| 继续沿用 `REMOTE_ONLY` 的纯 API 边界 | 避免新增本地模型分支，和用户当前目标一致 |
| 语言选项使用显式枚举而不是自由文本 | 便于持久化、迁移、提示词约束和测试 |
| 双语格式由统一提示词和结果约束保证 | 避免各调用点自行拼接导致格式不一致 |
| 透明度设置与现有配置存储绑定 | 可在启动时恢复并在运行时同步窗口 |
| 知识库操作区分索引删除与文件删除 | 保护用户文件，确保数据库和磁盘状态可验证一致 |

## Issues Encountered

| Issue | Resolution |
|-------|------------|
| 初始基线命令中的 `rg` 正则表达式错误 | 使用 `Select-String` 重跑，得到所需行号 |
| Rust 初次编译耗时且第二次检查遇到构建目录锁 | 等待增量编译完成，最终检查通过 |

## Resources

- 远端仓库：<https://github.com/hanhan761/forbb>
- 本地纯 API 构建模式：`src/lib/buildMode.ts`
- 本地 Rust feature 配置：`src-tauri/Cargo.toml`
- 产品说明：`README.md`

## Visual/Browser Findings

- GitHub 仓库页面显示项目是 Tauri 2 + React/TypeScript 的 AI 会议/面试助手，README 的当前版本说明纯 API 构建默认走远程 AI 服务。

### 2026-09-16 纯 API 触发链路确认

- `start_capture_per_party` 的 Qwen/云端 `Them` 转写路径在 `audio_commands.rs` 中直接调用 `engine.push_transcript(...)`，但没有把返回的 `DetectedQuestion` 转成 `question_detected` 事件。
- 传统 `push_transcript` IPC 命令会发出 `question_detected`，但纯 API 音频转写不经过该命令，因此自动回答只能依赖悬浮窗 `QuestionDetector` 对 `transcript_final` 的二次扫描。
- 会议启动时音频捕获早于悬浮窗事件监听建立，这使得转写/问题事件存在时序丢失窗口；这与“翻译有结果、回答需要按空格”的现象直接吻合。
- `Them` 终止时的 accumulator flush 路径同样直接 push、未发问题事件；旧的 system-audio STT 路径也有同样缺口。

### 2026-09-16 翻译与透明度链路补充

- `translate_segments` 当前直接对 `segment_ids.iter().zip(texts.iter())` 处理，没有校验两个数组长度一致；调用方虽然通常成对传值，但该边界会静默丢段，需改成显式校验。
- 自动翻译前端按 segment id 做 200ms debounce，但请求发出后没有保存“最新原文/目标语言”快照；同一个 id 的修订分段或切换语言期间，旧结果仍可能覆盖新结果。结果接收端也没有按当前 transcript 原文校验。
- LLM 翻译通过 provider mutex 后再监听 `llm_stream_token`，现有设计已避免捕获前一个回答流；本轮重点放在 segment id、原文和目标语言的一致性，而不是重写 Qwen 翻译流。
- `overlayOpacity` 已在配置 store 中持久化、跨窗口同步，GeneralSettings 也已有滑块；当前 Tauri JS API 没有可用的 `setOpacity`，因此实现为透明窗口上的 overlay 外层 DOM alpha，并通过配置 store 实时同步。
- 当前安装的 `@tauri-apps/api` 2.x 类型没有 `setOpacity` API，Tauri 配置允许透明背景；本轮不引入未经验证的 Windows 原生窗口 API，而是通过 overlay 外层页面背景 alpha 实时同步，避免调用不存在的 JS API。
- `translate_via_llm` 的 token 监听在 provider mutex 持有期间建立；翻译流本身有串流隔离，但结果缺少前端“当前 segment 原文/语言版本”校验。
- 默认 action prompt（Assist、WhatToSay、Shorten、FollowUp）还写死 `Respond only in Simplified Chinese`；仅修改后置指令会留下冲突，需移除这些模板中的写死语言并由回答语言配置统一注入。

### 2026-09-16 实现边界

- `LauncherMeetingHooks` 已经是音频、转写、翻译和流式回答的常驻 owner；新增自动回答 hook 放在这里可避开 overlay 的挂载时序。`QuestionDetector` 保留为悬浮窗展示/手动回答入口。
- `ContextManager` 的副本文件命名为 `<uuid>_<原文件名>`，因此批量/文件夹删除可以只依据受管资源 id 删除副本，不需要也不应该删除来源目录。
- 当前 DB migration 是幂等函数链，没有 user_version；新增 `v7_context_source_folder` 并让旧记录默认 `source_folder=''` 可保持兼容。
- `context_resources` 的 RAG 索引删除已有 `delete_chunks_by_file`，新增批量删除命令可以复用它，并在命令层统一处理资源、索引和内存 manager，避免 UI 再异步补删造成竞态。
- 当前透明度滑块与 DOM alpha 已存在，本轮将增加 overlay 背景色/根节点同步和清晰的配置边界；不引入不存在的 Tauri `setOpacity` 调用。

### 2026-09-16 验证补充

- 自动回答已从 overlay 展示组件移到 launcher 常驻 hook；纯 API Qwen 音频最终 `Them` 段现在统一进入问题检测并发出 `question_detected`，不再依赖空格或 overlay 挂载时序。
- 知识库删除已改为先校验、先清理 SQLite/RAG，再移除应用副本；来源目录只作为分组元数据，清理知识库时保留 transcript 类型的 RAG 行。
- `cargo test --locked --manifest-path src-tauri/Cargo.toml` 通过 70/70；`npm run build` 通过。真实 Qwen 网络请求和 Windows 桌面手工交互仍需在有 API key 的运行环境做最终冒烟验证。

### 2026-09-16 交付检查

- 主界面实际渲染 `ContextResourceList`，支持全选、批量删除、按来源目录删除；目录替换先导入新目录，再删除旧应用副本和索引。
- 动态回答语言提示词只在后端统一注入，`zh`、`en`、`bilingual` 三种设置均有测试覆盖；中英双语明确要求中文段在前、英文段在后。
- 全量 `cargo fmt --check` 的失败来自仓库原有无关格式差异，本轮未格式化全树；构建、测试和 diff 检查均通过。

### 2026-09-17 用户复验反馈

- 用户截图中的 `Sources` 区域仍没有明显的批量删除入口，且资源卡片没有选择框；当前源码虽包含旧版 `ContextResourceList`，但“删除选中”只有选中后才出现，“全选”使用 10px 弱化文字，无法作为可发现的批量管理入口。
- 当前 `dist` bundle 能检索到旧版批量删除字符串，说明源码已进入构建，但用户打开的窗口仍可能是旧安装包或旧构建；新入口必须通过始终可见、带边框和明确中文文案的管理栏验证，而不能只依赖隐藏式卡片操作。
- 用户最新截图确认透明度问题发生在 Windows 悬浮窗层：现有实现只设置 DOM/root 的 CSS alpha，Tauri 原生窗口及其子区域仍按不透明窗口合成。需要调用 Windows `SetLayeredWindowAttributes` 对 overlay 顶层 HWND 设置 alpha，并继续保留 CSS 透明背景以避免双重底色。
- 回答语言的枚举、设置持久化和后端 bilingual 提示词已经存在，但默认状态仍为 `zh`；自动回答和手动回答大多依赖 IPC wrapper 从 store 回退读取，用户未主动切换时会持续看到中文单语。此次将默认改为 `bilingual`，同时在回答面板展示当前语言状态，后端继续约束“中文分区在前、English 分区在后”。

### 2026-09-17 串题根因与修复

- 用户截图的回答元数据是 `quick_answer`，但答案仍复述 CSI-Bench，说明问题不是路由把该问题送入论文检索，而是 `generate_assist` 原先无条件把 `ContextManager::get_hot_context()` 放进每次 prompt。
- `get_hot_context()` 在没有高置信个人资料文件时还会回退到第一个资源；当第一个资源是“CSI-Bench 论文复现项目.md”时，论文正文会成为所有问题的隐式背景。
- 现在只有 `SearchFiles` 路由允许文件热上下文；`QuickAnswer`、`AskCodex` 和 `SearchWeb` 不再注入任何常驻文件正文。论文、项目报告和笔记仍可由相关问题触发 RAG 检索。
- 热上下文候选也收窄为明确的简历/个人资料命名，移除了 `project`、`research`、`项目`、`研究` 等会误识别论文的宽泛关键词，并删除“首个文件回退”。
- 新增回归测试先在旧逻辑下失败，再在修复后通过；全量 Rust 测试为 74/74，前端构建和 Windows release/NSIS 构建均通过。

### 2026-09-17 交付产物

- 使用 Nature 写作工作流整理了 `docs/user-guide/knowledge-base-and-overlay.md`，按操作入口、步骤、预期结果和安全边界说明知识库、双语回答及整窗透明度，并从 Getting Started 加入链接。
- Windows 安装包已生成：`src-tauri/target/release/bundle/nsis/NexQ_2.20.14_x64-setup.exe`。
