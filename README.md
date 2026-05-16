# AI Report Factory

AI Report Factory 是一个本地优先的 AI 报告生成桌面工具，当前架构已重构为 **Vue 3 + TypeScript + Tauri 2 + Rust 内置后端**。软件不再依赖 Python、FastAPI、PyInstaller、Presenton、CosyVoice、Wan2.2 或 ComfyUI，默认即可在本机生成基础 PPTX、Word 解说稿、TXT/MD 讲稿、字幕和分镜 JSON。

## 功能特性

- md/txt 文档输入与报告主题输入
- Rust 读取文本并生成结构化报告计划
- OpenAI 兼容接口、Gemini、Ollama、本地规则兜底
- 本地生成 PPTX：标题页、目录页、章节/内容页、总结页
- 本地生成 DOCX 解说稿，同时输出 TXT/MD 讲稿
- 输出 SRT 字幕和 storyboard JSON
- 可选 TTS 旁白：Windows SAPI 本地音色、OpenAI 兼容 TTS、Fish Speech/小智克隆音色接口
- 可选 MP4 视频：完整安装包和 portable zip 内置 ffmpeg，勾选视频即可把字幕和旁白合成为 1080P MP4
- SQLite 保存任务历史、进度、错误和产物路径
- 本地文件中心：扫描 outputs、打开文件/目录、复制路径、预览文本类文件
- 配置中心：AI Provider、API Key、模型、Ollama、输出目录、可选 TTS/视频/ffmpeg
- 运行环境检测：Rust 核心、SQLite、输出目录、配置、ffmpeg/TTS 可选状态
- Windows release 使用 `windows_subsystem = "windows"`，双击 exe 不弹命令行窗口
- GitHub Actions 支持 v* tag 自动构建 Windows 完整安装包、便携式 exe/zip 并发布 Release

## 技术栈

- 前端：Vue 3、TypeScript、Vite
- 桌面：Tauri 2
- 内置后端：Rust commands
- 数据库：SQLite / rusqlite
- 网络：reqwest
- 文件与生成：Rust + Office OpenXML + zip
- 可选语音/视频：Windows SAPI / OpenAI-compatible TTS / Fish Speech-compatible TTS / ffmpeg

## 架构图

```text
Vue 3 UI
  │ invoke
Tauri 2 Commands
  ├─ Config：%LOCALAPPDATA%/AI Report Factory/config.json
  ├─ SQLite：任务历史与产物索引
  ├─ AI：OpenAI compatible / Gemini / Ollama / Local fallback
  ├─ Generator：PPTX / DOCX / TXT / MD / SRT / JSON / optional WAV / optional MP4
  └─ Files：outputs 扫描、打开、预览
```

## 截图

> 后续可在 `docs/images/` 补充工作台、生成页、历史记录、配置中心截图。

## 快速开始

```powershell
cd ai-report-factory
npm.cmd --prefix frontend install
npm.cmd --prefix frontend run type-check
cd src-tauri
cargo check
```

开发运行：

```powershell
npm.cmd --prefix frontend run desktop:dev
```

## 便携式 exe 与完整安装包打包

```powershell
pnpm dlx @tauri-apps/cli build --no-bundle
```

产物路径：

```text
src-tauri/target/release/ai-report-factory.exe
```

如果要给普通用户一个“不需要再下载其他工具”的安装包，先准备内置 ffmpeg，再构建 NSIS：

```powershell
.\scripts\prepare-ffmpeg.ps1
npm.cmd --prefix frontend run desktop:installer
```

安装包产物：

```text
src-tauri/target/release/bundle/nsis/AI Report Factory_0.3.1_x64-setup.exe
```

## GitHub Actions 自动发布

推送 v* tag 会触发 `.github/workflows/release.yml`：

```powershell
git tag v0.3.1
git push origin v0.3.1
```

工作流会在 Windows 上执行前端类型检查、Cargo 检查、准备内置 ffmpeg、`build --no-bundle` 和完整 NSIS 安装包构建，并上传：

- `AI-Report-Factory-windows-x64.exe`
- `AI-Report-Factory-windows-x64-setup.exe`
- `AI-Report-Factory-windows-x64-portable.zip`
- `SHA256SUMS.txt`

也可以在 GitHub Actions 页面使用 `workflow_dispatch` 手动触发构建测试，手动触发只上传 artifact，不创建 Release。

## 本地发布脚本

`scripts/publish-release.ps1` 可在本机完成打包、校验和 Release 资产准备。默认只做本地打包与资产生成，不推送远程：

```powershell
.\scripts\publish-release.ps1 -Version v0.3.1
```

打包并推送 tag：

```powershell
.\scripts\publish-release.ps1 -Version v0.3.1 -PushTag
```

打包、推送 tag、创建 GitHub Release 并上传资产：

```powershell
.\scripts\publish-release.ps1 -Version v0.3.1 -PushTag -CreateRelease -UploadAssets
```

指定发布说明：

```powershell
.\scripts\publish-release.ps1 -Version v0.3.1 -NotesFile .\RELEASE_NOTES.md -PushTag -CreateRelease -UploadAssets
```

脚本会生成 `release-assets/AI-Report-Factory-windows-x64.exe`、`release-assets/AI-Report-Factory-windows-x64-setup.exe`、`release-assets/AI-Report-Factory-windows-x64-portable.zip`、`SHA256SUMS.txt`、`RELEASE_NOTES.md`。不会上传 outputs、logs、storage、config 或 `.env`。创建/上传 Release 需要本机安装并登录 GitHub CLI：`gh auth login`。

## 配置说明

配置保存到用户可写目录，例如 Windows：

```text
%LOCALAPPDATA%/AI Report Factory/config.json
```

API Key 只保存在本地配置文件，不会写入仓库或 Release。配置损坏时会自动备份为 `.broken.json` 并恢复默认配置。

## 无 Python / 无服务器说明

当前主流程不安装 Python、不启动 FastAPI、不使用 PyInstaller、不打包 Python exe、不需要 Presenton/CosyVoice/Wan2.2/ComfyUI。PPTX、DOCX、讲稿、字幕、分镜 JSON 均由 Rust 内置后端本地生成。完整安装包会内置 ffmpeg，普通用户无需另装 ffmpeg 即可合成 MP4；Windows SAPI 可本地生成 WAV。OpenAI/Fish Speech 兼容 TTS 与“小智克隆音色”保留为可选增强，需要用户具备对应授权服务时再启用。

## 路线图

- v0.3：去 Python、Rust 内置后端、配置中心、SQLite 历史、基础生成
- v0.4：增强 PPTX/DOCX 模板、本地文件管理、环境检测
- v0.5：可选 TTS、字幕、分镜 JSON、可选 ffmpeg 视频
- v1.0：稳定版、模板系统、自动更新、更多 AI Provider

## 开源协议

MIT
