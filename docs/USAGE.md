# 使用说明

## 如何运行

```powershell
npm.cmd --prefix frontend install
npm.cmd --prefix frontend run desktop:dev
```

## 如何配置 AI API

打开“配置中心”：

1. 选择 AI Provider：本地规则、OpenAI 兼容、Gemini 或 Ollama。
2. 填写 API Base URL、API Key、模型名称或 Ollama 地址。
3. 可保留“启用本地规则兜底”，AI 调用失败时仍能生成基础报告。
4. 点击“测试连接”。
5. 点击“保存配置”。

API Key 保存到 `%LOCALAPPDATA%/AI Report Factory/config.json`，不会提交到仓库。

## 如何生成第一个报告

1. 进入“新建生成”。
2. 上传 md/txt 文件，或直接粘贴文本。
3. 输入报告主题。
4. 选择输出内容：PPTX、DOCX、TXT/MD、字幕、分镜 JSON、语音旁白、1080P 视频。
5. 点击“开始生成”。
6. 进入任务详情查看进度和产物。

## 如何生成语音和视频

语音/视频是可选能力，不影响 PPTX、DOCX、讲稿、字幕和分镜 JSON 主流程。

1. 进入“配置中心”。
2. 开启“启用可选 TTS”，选择：
   - `Windows SAPI 本地语音`：不需要网络，使用系统已安装音色。
   - `OpenAI 兼容 TTS`：适合接入云端或本地兼容服务。
   - `Fish Speech / 小智克隆音色`：适合部署 Fish Speech 后，通过 voice 参数选择“小智克隆音色”。
3. 选择音色：Windows 默认、Huihui、Yaoyao、Kangkang、小智克隆音色、温暖女声、清晰男声等。
4. 如需 MP4，建议发布给用户 `AI-Report-Factory-windows-x64-setup.exe` 完整安装包，安装包内置 ffmpeg，用户不需要另装工具。
5. 新建任务时勾选“1080P 视频”。生成成功后任务详情会出现 `report_video.mp4`；如果使用单 exe 且未携带 ffmpeg，会给出明确错误提示。

## 如何查看历史记录

进入“历史记录”，可搜索、按状态筛选、查看详情、打开输出目录、删除记录或重新生成。

## 如何打开本地文件

进入“本地文件”，点击“扫描 outputs”，可打开文件、打开所在文件夹、复制路径；txt、md、json、log、srt、vtt 支持内置预览。

## 如何打包便携式 exe 和完整安装包

```powershell
pnpm dlx @tauri-apps/cli build --no-bundle
```

产物：

```text
src-tauri/target/release/ai-report-factory.exe
```

完整安装包不要求最终用户下载 ffmpeg：

```powershell
.\scripts\prepare-ffmpeg.ps1
npm.cmd --prefix frontend run desktop:installer
```

产物：

```text
src-tauri/target/release/bundle/nsis/AI Report Factory_0.3.1_x64-setup.exe
```

## 如何通过 GitHub Actions 自动发布 Release

推送 v* tag：

```powershell
git tag v0.3.1
git push origin v0.3.1
```

`.github/workflows/release.yml` 会自动准备内置 ffmpeg，构建 Windows 便携式 exe、完整安装包和 portable zip，并在 GitHub Release 上传：

- `AI-Report-Factory-windows-x64.exe`
- `AI-Report-Factory-windows-x64-setup.exe`
- `AI-Report-Factory-windows-x64-portable.zip`
- `SHA256SUMS.txt`

## 如何手动触发 workflow_dispatch

在 GitHub 仓库页面进入 Actions，选择 “Release Windows”，点击 “Run workflow”。手动触发用于构建测试，会上传 artifact 供下载。

## 如何使用本地发布脚本

仅本地打包，不上传：

```powershell
.\scripts\publish-release.ps1 -Version v0.3.1
```

打包并推送 tag：

```powershell
.\scripts\publish-release.ps1 -Version v0.3.1 -PushTag
```

打包、推送 tag、创建 Release、上传资产：

```powershell
.\scripts\publish-release.ps1 -Version v0.3.1 -PushTag -CreateRelease -UploadAssets
```

指定更新说明：

```powershell
.\scripts\publish-release.ps1 -Version v0.3.1 -NotesFile .\RELEASE_NOTES.md -PushTag -CreateRelease -UploadAssets
```

脚本会读取 `CHANGELOG.md` 生成 `release-assets/RELEASE_NOTES.md`；如果指定 `-NotesFile`，则使用指定文件。Release 资产只包含 exe、setup.exe、portable zip、SHA256SUMS 和 RELEASE_NOTES，不包含 outputs、logs、storage、config、`.env`。如果未安装 gh CLI 或未登录，脚本会提示安装 gh 或执行 `gh auth login`。
