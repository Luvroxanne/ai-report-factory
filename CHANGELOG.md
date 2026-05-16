# Changelog

## v0.3.1

- 修复视频生成无声问题：勾选视频时强制生成旁白音频，并在 TTS 配置缺失或失败时自动回退 Windows SAPI；若最终无法生成音频则停止任务，避免输出无声视频。
- 修复 Windows 播放器不支持视频编码的问题：MP4 改为优先输出 H.264/AVC1 + AAC/MP4A，并增加验收测试防止回退到不兼容编码。
- 增强 AI 报告生成 Agent：接入 AI Provider 后仍会做本地质量增强，补齐至少 9 页结构、专业 bullet、完整口播稿和页面布局。
- 增加 PPT 模板选择：极光科技蓝、商务白金、翡翠培训、橙紫路演、紫色创意。
- 增强 PPT 页面类型：封面、执行摘要、目录、指标卡片、时间线、对比分析、流程方案、洞察卡片、行动建议、总结。
- 重新生成 Windows 完整安装包、免安装 portable zip、SHA256 校验和 Release Notes。

## v0.3.0

- 重构为 Vue 3 + TypeScript + Tauri 2 + Rust 内置后端。
- 移除 Python FastAPI sidecar、PyInstaller 和 Python 后端主流程。
- 新增本地配置、SQLite 任务历史、本地文件管理。
- 新增 Rust 本地 PPTX、DOCX、TXT/MD、SRT、分镜 JSON 生成。
- 新增 Windows portable exe GitHub Actions 自动发布流程。
