# FAQ

## 1. 为什么不再使用 Python？

为了减少运行时、端口服务、PyInstaller 打包和 sidecar 管理成本。Rust 内置后端可以直接随 Tauri exe 运行。

## 2. 为什么可以直接运行 exe？

核心能力已经迁移到 Rust commands，配置、数据库和输出文件都在用户可写目录，不需要安装 Python 或启动服务器。

## 3. 没有 Presenton 能不能生成 PPT？

可以。默认使用 Rust 本地生成基础 PPTX。

## 4. 没有 CosyVoice 能不能生成语音？

第一阶段默认不生成语音，但会生成 Word 解说稿和 TXT/MD 讲稿。语音后续作为可选能力。

## 5. 没有 Wan2.2/ComfyUI 能不能生成视频？

第一阶段不强制生成 mp4，但会生成字幕和分镜 JSON。视频后续可基于可选 ffmpeg 扩展。

## 6. API Key 保存在哪里？

`%LOCALAPPDATA%/AI Report Factory/config.json`，不会写入仓库或 Release。

## 7. 生成文件保存在哪里？

默认在 `%LOCALAPPDATA%/AI Report Factory/outputs/<task-id>/`，也可在配置中心修改输出目录。

## 8. 如何查看历史记录？

打开“历史记录”页面，数据来自本地 SQLite。

## 9. 如何删除任务记录？

在任务详情点击“删除记录”。该操作删除数据库记录，不强制删除产物文件。

## 10. 为什么 release 版本不弹命令行窗口？

`src-tauri/src/main.rs` 顶部启用了 `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`。

## 11. 为什么不再强依赖服务器？

前端通过 Tauri invoke 直接调用 Rust，不再启动 FastAPI。本地规则兜底也不需要网络。

## 12. 视频功能为什么改成可选？

Wan2.2/ComfyUI 依赖重，不适合作为绿色版 exe 主流程。PPT 和 Word 主流程应优先稳定。

## 13. 如何通过 GitHub Actions 自动打包发布？

执行：

```powershell
git tag v0.3.1
git push origin v0.3.1
```

推送 v* tag 后 release workflow 会构建并发布 exe。

## 14. 如何手动触发 workflow_dispatch？

进入 GitHub Actions 页面，选择 “Release Windows Portable”，点击 “Run workflow”。手动触发会上传 artifact 便于测试。
