# 知识库、单语言回答与悬浮窗使用说明

本文说明 NexQ 中三个容易混淆、但彼此独立的设置：知识库文件管理、回答输出语言，以及悬浮窗透明度。

## 1. 导入知识库

打开 Settings → Context（设置 → 上下文），在 **Knowledge Base Folder** 区域选择：

- **Choose Folder**：把文件夹中的 PDF、TXT、Markdown 和 DOCX 文件递归导入知识库。
- **Replace**：先导入并验证新文件夹；新文件夹至少成功导入一个文件后，才清理旧的应用内副本和索引。
- **Choose Vault**：选择 Obsidian Vault，导入其中可用的 Markdown 笔记。
- **Browse Files**：只选择一个或多个文件，不导入整个文件夹。

导入完成后，资源会显示在 **知识库文件 (n)** 区域。文件旁的 **Indexed**、**New** 或 **Not Indexed** 标签表示当前索引状态。

## 2. 删除知识库文件

在 **知识库文件 (n)** 标题右侧可以看到 **全选** 和 **批量删除**。如果按钮处于禁用状态，先勾选文件卡片左侧的复选框。

删除操作分为三种：

1. 勾选若干文件，点击 **批量删除**。
2. 在某个来源文件夹标题右侧点击 **删除此文件夹**。
3. 点击单个文件卡片右侧的删除图标，并再次确认。

确认后，NexQ 会删除应用管理的副本、资源记录和对应索引；不会删除你原始文件夹中的文件。若要更换整个知识库，优先使用 **Replace**，这样空文件夹或导入失败不会先清空现有资源。

## 3. 让回答不被某一篇论文带偏

知识库不是每个问题的默认背景：

- **Auto** 会根据问题决定是否检索本地文件。
- **Quick** 只回答当前问题和当前会话内容，不注入已导入论文的常驻正文。
- **My files** 才会主动使用本地知识库检索。
- 论文、项目报告和笔记通过相关问题触发检索；明确的简历/个人资料可以作为个人资料热上下文。

因此，若问题与 CSI-Bench 或其他论文无关，选择 **Quick** 可以得到最干净的直接回答；若问题确实询问你的项目、论文或笔记，选择 **My files**，或保留 **Auto** 让路由器判断。

## 4. 设置回答语言

打开 Settings → Qwen，在 **回答输出语言** 中选择 **中文** 或 **English**。

每次回答只输出一种语言，不会同时输出中文和 English。回答只保留直接答案，不追加 AI 的分析、推理或 `My Take` 段落。

## 5. 设置整个悬浮窗的透明度

打开 Settings → General，在 **Overlay Transparency** 滑块中调整透明度：

- 左侧 **Transparent**：更透明；
- 右侧 **Solid**：更不透明；
- 当前百分比会即时显示，范围为 10%–100%。

该设置同时作用于 Windows 悬浮窗本身和窗口内的内容，不需要重启会议。设置会被保存，下一次打开悬浮窗时继续使用。

## 6. 建议的日常流程

1. 先导入简历或个人介绍，再按需导入论文和项目笔记。
2. 普通面试问答使用 **Auto** 或 **Quick**。
3. 需要核对论文细节时切换到 **My files**。
4. 默认使用 **中文**；需要英文回答时，在 Qwen 设置中切换到 **English**。
5. 更换知识库时使用 **Replace**，不要手动删除原始文件夹。

## English quick reference

- **Choose Folder** imports a folder recursively.
- **Replace** imports the new folder before removing old app-managed copies and indexes.
- **Batch Delete** removes checked resources; source files are preserved.
- **Quick** answers without resident paper context; **My files** enables local retrieval.
- Set **回答输出语言** to **中文** or **English**; each answer uses only the selected language.
- **Overlay Transparency** applies to the complete native overlay window and is persisted.
