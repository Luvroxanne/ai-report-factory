<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { api, emptyConfig, type AppConfig, type LocalFileItem, type SystemStatusItem, type TaskRecord, type TaskStatus, type VoiceOption } from './api/client'

type Page = 'dashboard' | 'generate' | 'history' | 'detail' | 'files' | 'settings' | 'environment' | 'about'

const page = ref<Page>('dashboard')
const config = ref<AppConfig>(emptyConfig())
const tasks = ref<TaskRecord[]>([])
const files = ref<LocalFileItem[]>([])
const systemItems = ref<SystemStatusItem[]>([])
const voiceOptions = ref<VoiceOption[]>([])
const selectedTask = ref<TaskRecord | null>(null)
const preview = ref('')
const message = ref('')
const busy = ref(false)
const search = ref('')
const statusFilter = ref('')
const form = ref({
  title: 'AI Report Factory 本地优先改造报告',
  inputFile: '',
  inputText: '# 项目目标\n将桌面端重构为 Vue 3 + TypeScript + Tauri 2 + Rust 内置后端。\n\n## 核心要求\n- 本地生成 PPTX、DOCX、字幕和分镜 JSON\n- 可选生成语音旁白和 MP4 视频\n- 保存任务历史\n- GitHub Actions 自动发布 Windows 便携 exe',
  style: 'agent-pro',
  template: 'aurora-tech',
  outputs: ['pptx', 'docx', 'script', 'subtitle', 'json']
})

let pollTimer: number | null = null

const recentTasks = computed(() => tasks.value.slice(0, 5))
const providerLabel = computed(() => ({
  open_ai_compatible: 'OpenAI 兼容',
  gemini: 'Gemini',
  ollama: 'Ollama',
  local: '本地规则'
}[config.value.ai_provider]))
const selectedArtifacts = computed(() => selectedTask.value ? [
  ['PPTX', selectedTask.value.pptx_path],
  ['DOCX', selectedTask.value.docx_path],
  ['讲稿', selectedTask.value.script_path],
  ['语音', selectedTask.value.audio_path],
  ['视频', selectedTask.value.video_path],
  ['字幕', selectedTask.value.subtitle_path],
  ['分镜 JSON', selectedTask.value.json_path],
  ['日志', selectedTask.value.log_path]
].filter(([, path]) => Boolean(path)) as Array<[string, string]> : [])

function statusText(status: TaskStatus) {
  return ({ pending: '等待中', running: '生成中', success: '成功', failed: '失败', cancelled: '已取消' } as Record<TaskStatus, string>)[status]
}

function setPage(next: Page) {
  page.value = next
  message.value = ''
  if (next === 'files') refreshFiles()
  if (next === 'environment') refreshSystem()
}

async function boot() {
  await Promise.all([loadConfig(), refreshTasks(), refreshSystem(), loadVoices()])
  startPolling()
}

async function loadConfig() {
  config.value = await api.getConfig()
}

async function loadVoices() {
  voiceOptions.value = await api.listTtsVoices().catch(() => [])
}

async function saveConfig() {
  busy.value = true
  try {
    config.value = await api.saveConfig(config.value)
    message.value = '配置已保存到用户可写目录'
  } catch (err) {
    message.value = err instanceof Error ? err.message : String(err)
  } finally {
    busy.value = false
  }
}

async function resetConfig() {
  config.value = await api.resetConfig()
  message.value = '已恢复默认配置'
}

async function testAi() {
  busy.value = true
  try {
    const result = await api.testAi(config.value)
    message.value = `${result.ok ? '连接可用' : '连接失败'}：${result.message}`
  } finally {
    busy.value = false
  }
}

async function onFileChange(event: Event) {
  const file = (event.target as HTMLInputElement).files?.[0]
  if (!file) return
  form.value.inputFile = file.name
  form.value.inputText = await file.text()
}

async function createTask() {
  busy.value = true
  message.value = ''
  try {
    const task = await api.createTask({
      title: form.value.title,
      input_file: form.value.inputFile,
      input_text: form.value.inputText,
      style: form.value.style,
      template: form.value.template,
      outputs: form.value.outputs
    })
    selectedTask.value = task
    page.value = 'detail'
    await refreshTasks()
  } catch (err) {
    message.value = err instanceof Error ? err.message : String(err)
  } finally {
    busy.value = false
  }
}

async function refreshTasks() {
  tasks.value = await api.listTasks(search.value || undefined, statusFilter.value || undefined)
  if (selectedTask.value) {
    selectedTask.value = tasks.value.find((item) => item.id === selectedTask.value?.id) || selectedTask.value
  }
}

async function refreshFiles() {
  files.value = await api.scanFiles()
}

async function refreshSystem() {
  systemItems.value = await api.systemStatus()
}

async function openTask(task: TaskRecord) {
  selectedTask.value = await api.getTask(task.id)
  preview.value = ''
  page.value = 'detail'
}

async function removeTask(task: TaskRecord) {
  await api.deleteTask(task.id)
  if (selectedTask.value?.id === task.id) selectedTask.value = null
  await refreshTasks()
}

async function rerun(task: TaskRecord) {
  selectedTask.value = await api.rerunTask(task.id)
  page.value = 'detail'
  await refreshTasks()
}

async function showPreview(path: string) {
  preview.value = await api.previewFile(path)
}

async function copyPath(path: string) {
  await navigator.clipboard?.writeText(path)
  message.value = '路径已复制'
}

function startPolling() {
  if (pollTimer) window.clearInterval(pollTimer)
  pollTimer = window.setInterval(refreshTasks, 1500)
}

onMounted(() => {
  boot().catch((err) => { message.value = err instanceof Error ? err.message : String(err) })
})
onBeforeUnmount(() => {
  if (pollTimer) window.clearInterval(pollTimer)
})
</script>

<template>
  <main class="app-shell">
    <aside class="sidebar">
      <div class="brand">
        <span class="brand-mark">AR</span>
        <div>
          <strong>AI Report Factory</strong>
          <small>Local-first Desktop</small>
        </div>
      </div>
      <nav>
        <button :class="{ active: page === 'dashboard' }" @click="setPage('dashboard')">工作台</button>
        <button :class="{ active: page === 'generate' }" @click="setPage('generate')">新建生成</button>
        <button :class="{ active: page === 'history' }" @click="setPage('history')">历史记录</button>
        <button :class="{ active: page === 'files' }" @click="setPage('files')">本地文件</button>
        <button :class="{ active: page === 'settings' }" @click="setPage('settings')">配置中心</button>
        <button :class="{ active: page === 'environment' }" @click="setPage('environment')">运行环境</button>
        <button :class="{ active: page === 'about' }" @click="setPage('about')">关于</button>
      </nav>
      <div class="side-card">
        <span>AI Provider</span>
        <b>{{ providerLabel }}</b>
        <small>{{ config.model_name || 'local-rule' }}</small>
      </div>
    </aside>

    <section class="content">
      <header class="topbar">
        <div>
          <p class="eyebrow">Vue 3 + TypeScript + Tauri 2 + Rust</p>
          <h1>本地优先的 AI 报告生成桌面工具</h1>
        </div>
        <button class="primary" @click="setPage('generate')">生成报告</button>
      </header>
      <p v-if="message" class="toast">{{ message }}</p>

      <section v-if="page === 'dashboard'" class="page-grid">
        <article class="hero-panel span-2">
          <p class="eyebrow">No Python · No Server Required</p>
          <h2>直接生成 PPTX、Word 解说稿、字幕、分镜 JSON 与可选 MP4</h2>
          <p>Rust 内置后端负责配置、SQLite 历史、本地文件、AI Provider、TTS 旁白和 Office/OpenXML/ffmpeg 产物生成。</p>
          <div class="quick-actions">
            <button class="primary" @click="setPage('generate')">新建报告</button>
            <button @click="setPage('settings')">配置 AI</button>
            <button @click="setPage('files')">查看 outputs</button>
          </div>
        </article>
        <article class="metric"><span>最近任务</span><b>{{ tasks.length }}</b><small>SQLite 本地保存</small></article>
        <article class="metric"><span>输出目录</span><b>outputs</b><small>{{ config.output_dir }}</small></article>
        <article v-for="cap in ['PPTX 本地生成', 'DOCX 解说稿', 'TXT/MD 讲稿', 'SRT 字幕', '分镜 JSON', '可选 TTS/MP4']" :key="cap" class="capability">{{ cap }}</article>
        <article class="panel span-2">
          <h3>最近任务</h3>
          <button v-for="task in recentTasks" :key="task.id" class="task-row" @click="openTask(task)">
            <span>{{ task.title }}</span><em :class="task.status">{{ statusText(task.status) }} · {{ task.progress }}%</em>
          </button>
        </article>
      </section>

      <section v-else-if="page === 'generate'" class="panel form-panel">
        <h2>新建生成</h2>
        <div class="form-grid">
          <label>报告主题<input v-model="form.title" /></label>
          <label>报告风格<select v-model="form.style"><option value="agent-pro">专业 Agent 增强</option><option value="official-tech">正式科技</option><option value="training">培训课件</option><option value="roadshow">路演汇报</option></select></label>
          <label>PPT 模板<select v-model="form.template"><option value="aurora-tech">极光科技蓝</option><option value="ivory-business">商务白金</option><option value="emerald-training">翡翠培训</option><option value="sunset-roadshow">橙紫路演</option><option value="violet-creative">紫色创意</option></select></label>
          <label class="span-2">上传 md/txt<input type="file" accept=".md,.txt" @change="onFileChange" /></label>
          <label class="span-2">输入内容<textarea v-model="form.inputText" rows="12" /></label>
        </div>
        <div class="checks">
          <label v-for="item in [['pptx','PPTX'],['docx','DOCX'],['script','TXT/MD 讲稿'],['subtitle','字幕'],['json','分镜 JSON'],['audio','语音旁白'],['video','1080P 视频']]" :key="item[0]">
            <input v-model="form.outputs" type="checkbox" :value="item[0]" /> {{ item[1] }}
          </label>
        </div>
        <p class="hint-card">完整安装包会内置 ffmpeg；勾选“1080P 视频”即可合成 MP4。音色仍在“配置中心”选择，默认使用 Windows 本地语音，不需要额外下载。</p>
        <button class="primary" :disabled="busy" @click="createTask">{{ busy ? '提交中...' : '开始生成' }}</button>
      </section>

      <section v-else-if="page === 'history'" class="panel">
        <div class="toolbar">
          <input v-model="search" aria-label="搜索任务" @keyup.enter="refreshTasks" />
          <select v-model="statusFilter" @change="refreshTasks"><option value="">全部状态</option><option value="pending">等待中</option><option value="running">生成中</option><option value="success">成功</option><option value="failed">失败</option></select>
          <button @click="refreshTasks">刷新</button>
        </div>
        <button v-for="task in tasks" :key="task.id" class="task-row rich" @click="openTask(task)">
          <span><b>{{ task.title }}</b><small>{{ task.created_at }}</small></span>
          <em :class="task.status">{{ statusText(task.status) }} · {{ task.current_step }}</em>
          <i>{{ task.output_dir }}</i>
        </button>
      </section>

      <section v-else-if="page === 'detail'" class="panel">
        <template v-if="selectedTask">
          <div class="detail-head">
            <div><h2>{{ selectedTask.title }}</h2><p>{{ selectedTask.current_step }} · {{ selectedTask.progress }}%</p></div>
            <button v-if="selectedTask.output_dir" @click="api.openInFolder(selectedTask.output_dir)">打开目录</button>
          </div>
          <div class="progress"><span :style="{ width: `${selectedTask.progress}%` }" /></div>
          <p v-if="selectedTask.error_message" class="error">{{ selectedTask.error_message }}</p>
          <div class="artifact-grid">
            <article v-for="[name, path] in selectedArtifacts" :key="path">
              <b>{{ name }}</b><small>{{ path }}</small>
              <div><button @click="api.openPath(path)">打开</button><button @click="api.openInFolder(path)">所在文件夹</button><button @click="copyPath(path)">复制路径</button><button v-if="/\\.(txt|md|json|log|srt|vtt)$/i.test(path)" @click="showPreview(path)">预览</button></div>
            </article>
          </div>
          <div class="detail-actions"><button @click="rerun(selectedTask)">重新生成</button><button class="danger" @click="removeTask(selectedTask)">删除记录</button></div>
          <pre v-if="preview" class="preview">{{ preview }}</pre>
        </template>
        <p v-else>请选择一个任务。</p>
      </section>

      <section v-else-if="page === 'files'" class="panel">
        <div class="toolbar"><h2>本地文件</h2><button @click="refreshFiles">扫描 outputs</button></div>
        <article v-for="file in files" :key="file.path" class="file-row">
          <b>{{ file.name }}</b><span>{{ file.file_type }} · {{ (file.size / 1024).toFixed(1) }} KB</span><small>{{ file.path }}</small>
          <div><button @click="api.openPath(file.path)">打开</button><button @click="api.openInFolder(file.path)">所在文件夹</button><button @click="copyPath(file.path)">复制路径</button><button v-if="file.previewable" @click="showPreview(file.path)">预览</button></div>
        </article>
        <pre v-if="preview" class="preview">{{ preview }}</pre>
      </section>

      <section v-else-if="page === 'settings'" class="panel form-panel">
        <h2>配置中心</h2>
        <div class="form-grid">
          <label>AI Provider<select v-model="config.ai_provider"><option value="local">本地规则</option><option value="open_ai_compatible">OpenAI 兼容</option><option value="gemini">Gemini</option><option value="ollama">Ollama</option></select></label>
          <label>模型名称<input v-model="config.model_name" /></label>
          <label>API Base URL<input v-model="config.api_base_url" /></label>
          <label>API Key<input v-model="config.api_key" type="password" aria-label="API Key 仅保存到用户目录" /></label>
          <label>Ollama 地址<input v-model="config.ollama_url" /></label>
          <label>输出目录<input v-model="config.output_dir" /></label>
          <label>TTS Provider<select v-model="config.tts_provider"><option value="none">不生成语音</option><option value="windows_sapi">Windows SAPI 本地语音</option><option value="open_ai_compatible">OpenAI 兼容 TTS</option><option value="fish_speech">Fish Speech / 小智克隆音色</option></select></label>
          <label>音色<select v-model="config.tts_voice"><option v-for="voice in voiceOptions" :key="voice.id" :value="voice.id">{{ voice.label }} · {{ voice.provider }}</option></select></label>
          <label>TTS Base URL<input v-model="config.tts_base_url" /></label>
          <label>TTS API Key<input v-model="config.tts_api_key" type="password" aria-label="TTS API Key 仅保存到用户目录" /></label>
          <label>TTS 模型<input v-model="config.tts_model" /></label>
          <label>ffmpeg 路径<input v-model="config.ffmpeg_path" aria-label="ffmpeg 路径，留空则自动查找内置 ffmpeg" /></label>
          <label>视频宽度<input v-model.number="config.video_width" type="number" min="640" step="10" /></label>
          <label>视频高度<input v-model.number="config.video_height" type="number" min="360" step="10" /></label>
          <label>视频 FPS<input v-model.number="config.video_fps" type="number" min="12" max="60" /></label>
        </div>
        <div class="checks">
          <label><input v-model="config.enable_local_fallback" type="checkbox" /> 启用本地规则兜底</label>
          <label><input v-model="config.enable_tts" type="checkbox" /> 启用可选 TTS</label>
          <label><input v-model="config.enable_video" type="checkbox" /> 启用可选视频</label>
          <label><input v-model="config.enable_ffmpeg" type="checkbox" /> 启用可选 ffmpeg</label>
        </div>
        <p class="hint-card">视频合成条件：新建生成时勾选“1080P 视频”。完整安装包和 portable zip 会内置 tools/ffmpeg/ffmpeg.exe；ffmpeg 路径留空会自动查找内置文件。音色在这里选择，默认 Windows SAPI 本地语音不需要下载。</p>
        <div class="quick-actions"><button class="primary" :disabled="busy" @click="saveConfig">保存配置</button><button @click="testAi">测试连接</button><button @click="resetConfig">恢复默认</button></div>
      </section>

      <section v-else-if="page === 'environment'" class="page-grid">
        <article v-for="item in systemItems" :key="item.name" class="metric">
          <span>{{ item.name }}</span><b :class="{ good: item.ok }">{{ item.ok ? '可用' : '可选/异常' }}</b><small>{{ item.detail }}</small>
        </article>
      </section>

      <section v-else class="panel about">
        <h2>关于 AI Report Factory</h2>
        <p>AI Report Factory 是本地优先的 AI 报告生成桌面工具，技术栈为 Vue 3、TypeScript、Tauri 2 与 Rust 内置后端。</p>
        <ul>
          <li>GitHub：github.com/Luvroxanne/ai-report-factory</li>
          <li>版本：v0.3.1</li>
          <li>许可证：MIT</li>
          <li>路线图：模板系统、更多 AI Provider、可选 TTS/ffmpeg 视频、自动更新。</li>
        </ul>
      </section>
    </section>
  </main>
</template>
