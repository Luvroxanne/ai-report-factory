export type TaskStatus =
  | 'pending'
  | 'parsing'
  | 'generating_ppt'
  | 'generating_script'
  | 'generating_voice'
  | 'generating_video'
  | 'completed'
  | 'failed'

export interface TaskView {
  id: string
  original_filename: string
  status: TaskStatus
  current_step: string
  progress: number
  project_dir?: string | null
  ppt_path?: string | null
  script_path?: string | null
  video_path?: string | null
  json_path?: string | null
  audio_dir?: string | null
  subtitle_path?: string | null
  log_path?: string | null
  metadata_path?: string | null
  error?: string | null
  created_at: string
  updated_at: string
}

export interface DependencyItem {
  name: string
  ok: boolean
  detail: string
}

export interface ProviderTestResult {
  ok: boolean
  provider: string
  message: string
}

function resolveApiBase(): string {
  const configured = import.meta.env.VITE_API_BASE
  if (configured) return String(configured).replace(/\/$/, '')

  // Vite 开发模式下通过 dev server proxy 转发 /api；
  // Tauri 正式包里页面也可能是 http(s)://tauri.localhost，此时不能用相对路径，
  // 否则 /api 会命中前端静态资源服务并返回 index.html。
  const isViteDevServer =
    ['localhost', '127.0.0.1', '[::1]'].includes(window.location.hostname) &&
    window.location.port === '5173'
  return isViteDevServer ? '' : 'http://127.0.0.1:8000'
}

const API_BASE = resolveApiBase()

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, ms))
}

function isNetworkFetchError(error: unknown): boolean {
  return error instanceof TypeError && /fetch|network|failed/i.test(error.message)
}

async function request<T>(url: string, init?: RequestInit): Promise<T> {
  let response: Response | null = null
  let lastError: unknown = null
  const maxAttempts = API_BASE ? 40 : 3

  for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
    try {
      response = await fetch(`${API_BASE}${url}`, init)
      break
    } catch (err) {
      lastError = err
      if (!isNetworkFetchError(err) || attempt >= maxAttempts) break
      await sleep(750)
    }
  }

  if (!response) {
    throw new Error(
      `连接本地后端失败，请稍等或重启应用。` +
        `如果是打包版，请确认本地后端已启动且未被防火墙拦截。` +
        `${lastError instanceof Error ? ` 原始错误：${lastError.message}` : ''}`
    )
  }

  const contentType = response.headers.get('content-type') || ''

  if (!response.ok) {
    const payload = contentType.includes('application/json') ? await response.json().catch(() => ({})) : {}
    throw new Error(payload.detail || payload.message || `请求失败：${response.status}`)
  }

  if (!contentType.includes('application/json')) {
    const preview = await response.text().catch(() => '')
    throw new Error(`后端返回了非 JSON 内容，请检查后端服务是否启动。响应开头：${preview.slice(0, 80)}`)
  }

  return response.json()
}

export async function waitForBackendReady(timeoutMs = 45000): Promise<void> {
  const deadline = Date.now() + timeoutMs
  let lastError = ''
  while (Date.now() < deadline) {
    try {
      await request('/api/health')
      return
    } catch (err) {
      lastError = err instanceof Error ? err.message : String(err)
      await sleep(900)
    }
  }
  throw new Error(`本地后端启动超时：${lastError}`)
}

export async function createTask(file: File, style: string): Promise<{ task_id: string; status: TaskStatus }> {
  const form = new FormData()
  form.append('file', file)
  form.append('style', style)
  return request('/api/tasks', {
    method: 'POST',
    body: form
  })
}

export async function listTasks(): Promise<TaskView[]> {
  return request('/api/tasks')
}

export async function getTask(taskId: string): Promise<TaskView> {
  return request(`/api/tasks/${taskId}`)
}

export async function getConfig(): Promise<Record<string, unknown>> {
  const payload = await request<{ config: Record<string, unknown> }>('/api/config')
  return payload.config
}

export async function saveConfig(config: Record<string, unknown>): Promise<Record<string, unknown>> {
  const payload = await request<{ config: Record<string, unknown> }>('/api/config', {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ config })
  })
  return payload.config
}

export async function testProvider(provider: string, config: Record<string, unknown>): Promise<ProviderTestResult> {
  return request('/api/config/test', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ provider, config })
  })
}

export async function getDependencies(): Promise<DependencyItem[]> {
  const payload = await request<{ items: DependencyItem[] }>('/api/dependencies')
  return payload.items
}

export function downloadUrl(taskId: string, kind: 'ppt' | 'script' | 'video' | 'json' | 'subtitle' | 'log' | 'metadata'): string {
  return `${API_BASE}/api/tasks/${taskId}/download/${kind}`
}
