# 架构说明

## 总体架构

AI Report Factory 采用本地优先桌面架构：Vue 3 前端负责交互，Tauri 2 负责桌面壳与安全边界，Rust commands 作为内置后端提供配置、任务、文件、数据库、AI 接口调用和本地产物生成能力。

```text
用户界面(Vue) -> Tauri invoke -> Rust commands -> SQLite / 文件系统 / AI Provider / 生成器
```

## 前端架构

前端位于 `frontend/`：

- `src/App.vue`：多 Tab 桌面应用壳，包含工作台、新建生成、历史、任务详情、本地文件、配置中心、运行环境、关于。
- `src/api/client.ts`：封装 Tauri invoke 调用和类型定义。
- `src/views/`：预留页面拆分目录，当前核心页面逻辑集中在 `App.vue`。

前端不再通过 HTTP 调用 `127.0.0.1:8000`，也不再依赖 Vite proxy 到 FastAPI。

## Rust 内置后端架构

后端位于 `src-tauri/src/`：

- `commands/`：Tauri commands，包括配置、任务、文件、AI 测试、系统检测。
- `config/`：配置文件读写、默认值、损坏恢复。
- `db/`：SQLite 迁移和任务模型。
- `services/ai/`：OpenAI 兼容、Gemini、Ollama、本地规则兜底。
- `services/ppt/`：基础 PPTX OpenXML 生成器。
- `services/docx/`：DOCX、Markdown、TXT 讲稿生成器。
- `services/files/`：outputs 扫描、预览安全检查。
- `services/tasks/`：任务运行器和状态常量。
- `services/optional/tts.rs`：Windows SAPI、OpenAI 兼容 TTS、Fish Speech/小智克隆音色接口。
- `services/optional/video.rs`：ffmpeg MP4 合成，可将字幕和旁白合入视频。
- `utils/`：路径和错误处理。

## 数据库设计

SQLite 文件：

```text
%LOCALAPPDATA%/AI Report Factory/storage/ai_report_factory.sqlite3
```

`tasks` 字段包括：id、title、input_file、input_text、status、progress、current_step、output_dir、pptx_path、docx_path、script_path、subtitle_path、json_path、log_path、created_at、updated_at、error_message。

状态：`pending`、`running`、`success`、`failed`、`cancelled`。

## 任务流程

1. 前端读取 md/txt 文件内容或用户输入文本。
2. 调用 `create_task` 写入 SQLite。
3. Rust 后台线程读取配置和输入内容。
4. AI Provider 生成结构化计划，失败时按配置降级到本地规则。
5. 生成 PPTX、DOCX、TXT/MD、SRT、storyboard JSON。
6. 如果开启 TTS，生成 `narration.wav`；如果开启视频且检测到 ffmpeg，生成 `report_video.mp4`。
7. 更新任务记录、产物路径和日志。
8. 前端轮询任务列表并展示详情。

## 文件目录

Windows 默认运行数据目录：

```text
%LOCALAPPDATA%/AI Report Factory/
├─ config.json
├─ storage/ai_report_factory.sqlite3
├─ outputs/<task-id>/
│  ├─ report.pptx
│  ├─ speaker_script.docx
│  ├─ speaker_script.md
│  ├─ speaker_script.txt
│  ├─ subtitle.srt
│  ├─ storyboard.json
│  └─ generation.log
└─ logs/
```

## 配置文件位置

`%LOCALAPPDATA%/AI Report Factory/config.json`。配置损坏时，旧文件会备份为 `.broken.json` 并自动写入默认配置。

## 生成流程

PPTX 和 DOCX 使用 Rust 直接写 Office OpenXML zip 包。第一版模板强调可打开、可演示、可维护，后续可扩展主题、母版、图表和模板替换。

## 无服务器设计

软件不启动本地 HTTP 服务，前端通过 Tauri invoke 调用 Rust；生成文件、数据库、配置均在用户目录中。OpenAI/Gemini/Ollama 是可配置外部能力，本地规则兜底保证无网络也能生成基础内容。

## 为什么不再使用 Python

Python FastAPI sidecar 会带来额外运行时、PyInstaller 打包体积、端口占用、命令行窗口和部署不确定性。迁移到 Rust 内置后端后，Windows 用户可以直接运行便携式 exe，主流程更轻、更稳定、更易发布。
