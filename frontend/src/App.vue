<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { createTask, downloadUrl, getConfig, getDependencies, getTask, listTasks, saveConfig, testProvider, waitForBackendReady, type DependencyItem, type TaskStatus, type TaskView } from './api/client'

type AppConfig = Record<string, any>
type ArtifactKind = 'ppt' | 'script' | 'video' | 'json' | 'subtitle' | 'log' | 'metadata'

const selectedFile = ref<File | null>(null)
const selectedStyle = ref('official-tech')
const loading = ref(false)
const task = ref<TaskView | null>(null)
const recentTasks = ref<TaskView[]>([])
const error = ref('')
const settingsOpen = ref(false)
const configDraft = ref<AppConfig>(emptyConfig())
const deps = ref<DependencyItem[]>([])
const savingConfig = ref(false)
const providerChecking = ref(false)
const providerResult = ref('')
let timer: number | null = null

const statusText: Record<TaskStatus, string> = {
  pending: '等待中', parsing: '整理材料', generating_ppt: '生成PPT', generating_script: '生成解说稿', generating_voice: '生成语音', generating_video: '合成视频', completed: '已完成', failed: '失败'
}
const statusLabel = computed(() => (task.value ? statusText[task.value.status] : '准备就绪'))
const activeProvider = computed({ get: () => String(configDraft.value.ai?.active_provider || 'local'), set: (value: string) => { ensureConfigShape(configDraft.value); configDraft.value.ai.active_provider = value } })
const activeProviderConfig = computed(() => providerConfig(activeProvider.value))
const presentonConfig = computed(() => serviceConfig('presenton'))
const cosyvoiceConfig = computed(() => serviceConfig('cosyvoice'))
const wanConfig = computed(() => serviceConfig('wan'))
const phaseSteps = [{ label: '材料解析', progress: 8 }, { label: 'PPT/大纲', progress: 42 }, { label: 'DOCX解说稿', progress: 56 }, { label: '语音片段', progress: 72 }, { label: '1080P视频', progress: 90 }, { label: '统一归档', progress: 100 }]
const artifactCards = computed<Array<{ kind: ArtifactKind; tag: string; title: string; desc: string; available: boolean }>>(() => [
  { kind: 'ppt', tag: 'PPTX', title: '商业PPT', desc: '封面、目录、章节、正文、总结页统一样式。', available: Boolean(task.value?.ppt_path) },
  { kind: 'script', tag: 'DOCX', title: 'Word解说稿', desc: '按页输出讲解词、预计时长与备注。', available: Boolean(task.value?.script_path) },
  { kind: 'video', tag: 'MP4', title: '1080P视频', desc: 'H.264编码，匹配音频节奏并带字幕。', available: Boolean(task.value?.video_path) },
  { kind: 'json', tag: 'JSON', title: '中间结构', desc: '保留可二次编辑的报告结构。', available: Boolean(task.value?.json_path) },
  { kind: 'subtitle', tag: 'SRT', title: '字幕文件', desc: '字幕内容来自每页解说稿。', available: Boolean(task.value?.subtitle_path) },
  { kind: 'log', tag: 'LOG', title: '生成日志', desc: '关键步骤、兜底原因与路径记录。', available: Boolean(task.value?.log_path) },
  { kind: 'metadata', tag: 'META', title: '元数据', desc: '产物路径、音频时长、脚本与任务信息。', available: Boolean(task.value?.metadata_path) }
])

function emptyConfig(): AppConfig { return { output_dir: 'outputs', ai: { active_provider: 'ollama', timeout_seconds: 90, retries: 2, providers: { openai: { base_url: 'https://api.openai.com/v1', api_key: '', model: 'gpt-4o-mini' }, gemini: { base_url: 'https://generativelanguage.googleapis.com', api_key: '', model: 'gemini-1.5-flash' }, ollama: { base_url: 'http://127.0.0.1:11434', model: 'qwen2.5:7b' }, local: { base_url: '', api_key: '', model: '' } } }, services: { presenton: { base_url: '', endpoint: '/api/v1/ppt/presentation/generate', username: '', password: '' }, cosyvoice: { base_url: '', endpoint: '/api/tts' }, wan: { base_url: '', mode: 'comfyui', workflow_template_path: '', poll_timeout_seconds: 600 } }, video: { width: 1920, height: 1080, fps: 24 }, desktop: { backend_url: 'http://127.0.0.1:8000' } } }
function isPlainObject(value: unknown): value is AppConfig { return Boolean(value && typeof value === 'object' && !Array.isArray(value)) }
function clonePlain<T>(value: T): T { return JSON.parse(JSON.stringify(value)) as T }
function mergeMissing(target: AppConfig, source: AppConfig) { for (const [key, value] of Object.entries(source)) { if (target[key] === undefined || target[key] === null) target[key] = clonePlain(value); else if (isPlainObject(target[key]) && isPlainObject(value)) mergeMissing(target[key], value) } }
function ensureConfigShape(config: AppConfig) { mergeMissing(config, emptyConfig()) }
function providerConfig(name: string): AppConfig { ensureConfigShape(configDraft.value); const providers = configDraft.value.ai.providers; if (!providers[name]) providers[name] = { base_url: '', api_key: '', model: '' }; return providers[name] }
function serviceConfig(name: string): AppConfig { ensureConfigShape(configDraft.value); const services = configDraft.value.services; if (!services[name]) services[name] = {}; return services[name] }
function clearMaskedSecrets(value: unknown) { if (!isPlainObject(value)) return; for (const [key, item] of Object.entries(value)) { if (['api_key', 'token', 'password', 'secret'].includes(key.toLowerCase())) value[key] = ''; else clearMaskedSecrets(item) } }
function stripBlankSecrets(value: unknown) { if (!isPlainObject(value)) return; for (const key of Object.keys(value)) { const item = value[key]; if (['api_key', 'token', 'password', 'secret'].includes(key.toLowerCase())) { if (!String(item || '').trim()) delete value[key] } else stripBlankSecrets(item) } }
function configForSubmit(): AppConfig { const payload = clonePlain(configDraft.value); stripBlankSecrets(payload); return payload }
function onFileChange(event: Event) { const input = event.target as HTMLInputElement; selectedFile.value = input.files?.[0] || null; error.value = '' }
async function submit() { if (!selectedFile.value) { error.value = '请先上传 Markdown 或 TXT 材料'; return } loading.value = true; error.value = ''; try { const created = await createTask(selectedFile.value, selectedStyle.value); task.value = { id: created.task_id, original_filename: selectedFile.value.name, status: created.status, current_step: '等待中', progress: 0, created_at: '', updated_at: '' }; startPolling(created.task_id) } catch (err) { error.value = err instanceof Error ? err.message : '创建任务失败' } finally { loading.value = false } }
function startPolling(taskId: string) { if (timer) window.clearInterval(timer); timer = window.setInterval(async () => { try { const latest = await getTask(taskId); task.value = latest; if (latest.status === 'completed' || latest.status === 'failed') { if (timer) window.clearInterval(timer); timer = null; recentTasks.value = await listTasks().catch(() => recentTasks.value) } } catch (err) { error.value = err instanceof Error ? err.message : '查询任务失败' } }, 1500) }
async function loadSettings() { const config = await getConfig(); const draft = clonePlain(config) as AppConfig; ensureConfigShape(draft); clearMaskedSecrets(draft); configDraft.value = draft }
async function persistSettings() { savingConfig.value = true; providerResult.value = ''; try { const saved = await saveConfig(configForSubmit()); const draft = clonePlain(saved) as AppConfig; ensureConfigShape(draft); clearMaskedSecrets(draft); configDraft.value = draft; providerResult.value = '配置已保存；Token 不会写入代码。' } catch (err) { providerResult.value = err instanceof Error ? err.message : '保存失败' } finally { savingConfig.value = false } }
async function checkProvider() { providerChecking.value = true; providerResult.value = ''; try { const result = await testProvider(activeProvider.value, configForSubmit()); providerResult.value = `${result.ok ? '可用' : '不可用'}：${result.message}` } catch (err) { providerResult.value = err instanceof Error ? err.message : '检测失败' } finally { providerChecking.value = false } }
async function refreshDependencies() { deps.value = await getDependencies().catch(() => []) }
async function startTauriBackend() { const invoke = (window as any).__TAURI_INTERNALS__?.invoke; if (invoke) await invoke('start_backend') }
onMounted(async () => { try { await startTauriBackend(); await waitForBackendReady() } catch (err) { error.value = err instanceof Error ? err.message : '本地后端启动失败' } await Promise.all([loadSettings().catch(() => undefined), refreshDependencies()]); recentTasks.value = await listTasks().catch(() => []) })
onBeforeUnmount(() => { if (timer) window.clearInterval(timer) })
</script>

<template>
  <main class="shell">
    <section class="hero">
      <div class="hero__copy">
        <p class="eyebrow">WINDOWS DESKTOP · AI REPORT FACTORY</p>
        <h1>把粗糙材料变成可交付的 PPT、解说稿和 1080P 成片</h1>
        <p class="lead">保留 MVP 兜底链路，同时升级为可配置、可观测、可打包的桌面应用。Presenton、CosyVoice、Wan2.2 可用时优先调用，不可用时自动进入本地兜底。</p>
        <div class="hero__actions"><button class="ghost-action" @click="settingsOpen = !settingsOpen">{{ settingsOpen ? '收起设置' : '打开能力配置' }}</button><span class="desktop-pill">Tauri + Rust 桌面壳</span></div>
      </div>
      <div class="radar-card"><div class="radar-card__ring"></div><div class="radar-card__center"><strong>{{ task?.progress ?? 0 }}%</strong><span>{{ statusLabel }}</span></div></div>
    </section>

    <section v-if="settingsOpen" class="settings-panel">
      <div class="panel__header"><span class="index">SET</span><div><h2>能力与 Token 配置</h2><p>Token 留空会保留已保存值；完全不配置也能用本地规则、Windows TTS 或静音兜底完整跑通。</p></div></div>
      <div class="settings-grid">
        <label><span>AI Provider</span><select v-model="activeProvider"><option value="openai">OpenAI兼容</option><option value="gemini">Gemini</option><option value="ollama">Ollama</option><option value="local">本地规则兜底</option></select></label>
        <label><span>输出目录</span><input v-model="configDraft.output_dir" placeholder="outputs 或绝对路径" /></label>
        <label v-if="activeProvider !== 'local'"><span>模型</span><input v-model="activeProviderConfig.model" placeholder="模型名称" /></label>
        <label v-if="activeProvider !== 'local'"><span>Base URL</span><input v-model="activeProviderConfig.base_url" placeholder="服务地址" /></label>
        <label v-if="activeProvider === 'openai' || activeProvider === 'gemini'"><span>API Token</span><input v-model="activeProviderConfig.api_key" type="password" placeholder="留空则保留已保存 Token" /></label>
        <label><span>Presenton 地址</span><input v-model="presentonConfig.base_url" placeholder="http://127.0.0.1:3000" /></label>
        <label><span>CosyVoice 地址</span><input v-model="cosyvoiceConfig.base_url" placeholder="http://127.0.0.1:9880" /></label>
        <label><span>Wan2.2/ComfyUI 地址</span><input v-model="wanConfig.base_url" placeholder="http://127.0.0.1:8188" /></label>
        <label><span>Wan2.2 工作流 JSON</span><input v-model="wanConfig.workflow_template_path" placeholder="可选：ComfyUI workflow 路径" /></label>
      </div>
      <div class="settings-actions"><button class="ghost-action" :disabled="providerChecking" @click="checkProvider">{{ providerChecking ? '检测中...' : '检测 Token/服务' }}</button><button class="primary-action compact" :disabled="savingConfig" @click="persistSettings">{{ savingConfig ? '保存中...' : '保存配置' }}</button><span v-if="providerResult" class="setting-result">{{ providerResult }}</span></div>
      <div class="deps"><button class="mini-button" @click="refreshDependencies">刷新依赖</button><span v-for="item in deps" :key="item.name" :class="['dep', { ok: item.ok }]">{{ item.name }} · {{ item.ok ? '可用' : '缺失' }}</span></div>
    </section>

    <section class="workspace">
      <div class="panel uploader"><div class="panel__header"><span class="index">01</span><div><h2>上传汇报材料</h2><p>支持 Markdown / TXT。无 Token 时使用本地模板生成基础 PPT、DOCX 解说稿、音频、字幕与视频。</p></div></div><label class="dropzone"><input type="file" accept=".md,.txt" @change="onFileChange" /><span class="dropzone__icon">＋</span><strong>{{ selectedFile?.name || '选择一份材料文件' }}</strong><small>建议包含标题、章节、要点；系统会保留中间 JSON 便于二次编辑。</small></label><div class="style-grid"><label :class="{ active: selectedStyle === 'official-tech' }"><input v-model="selectedStyle" type="radio" value="official-tech" /><span>科技政企</span></label><label :class="{ active: selectedStyle === 'training' }"><input v-model="selectedStyle" type="radio" value="training" /><span>培训课程</span></label><label :class="{ active: selectedStyle === 'roadshow' }"><input v-model="selectedStyle" type="radio" value="roadshow" /><span>路演提案</span></label></div><button class="primary-action" :disabled="loading" @click="submit">{{ loading ? '创建任务中...' : '开始生成最终产物' }}</button><p v-if="error" class="error">{{ error }}</p></div>
      <div class="panel flow"><div class="panel__header"><span class="index">02</span><div><h2>任务状态</h2><p>{{ task?.current_step || '等待新任务。所有关键步骤会写入日志，失败时自动进入可用兜底。' }}</p></div></div><ol class="steps"><li v-for="item in phaseSteps" :key="item.label" :class="{ done: (task?.progress ?? 0) >= item.progress }">{{ item.label }}</li></ol><div class="progress"><span :style="{ width: `${task?.progress ?? 0}%` }"></span></div><p class="current-step">{{ task?.id ? `任务ID：${task.id}` : '尚未创建任务' }}</p><p v-if="task?.error" class="error">{{ task.error }}</p></div>
    </section>

    <section class="result-grid"><article v-for="card in artifactCards" :key="card.kind" :class="['result-card', { ready: card.available }]"><span>{{ card.tag }}</span><h3>{{ card.title }}</h3><p>{{ card.desc }}</p><a v-if="task?.status === 'completed' && card.available" :href="downloadUrl(task.id, card.kind)">下载</a><small v-else>生成完成后可下载</small></article></section>
    <section v-if="recentTasks.length" class="history panel"><div class="panel__header"><span class="index">HIS</span><div><h2>最近任务</h2><p>选择最近任务可继续查看进度或下载产物。</p></div></div><button v-for="item in recentTasks" :key="item.id" class="history-row" @click="task = item"><span>{{ item.original_filename }}</span><b>{{ statusText[item.status] }}</b><em>{{ item.updated_at }}</em></button></section>
  </main>
</template>
