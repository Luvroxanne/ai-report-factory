export type AiProvider = 'open_ai_compatible' | 'gemini' | 'ollama' | 'local'
export type TtsProvider = 'none' | 'windows_sapi' | 'open_ai_compatible' | 'fish_speech'
export type TaskStatus = 'pending' | 'running' | 'success' | 'failed' | 'cancelled'

export interface AppConfig {
  ai_provider: AiProvider
  api_base_url: string
  api_key: string
  model_name: string
  ollama_url: string
  output_dir: string
  enable_tts: boolean
  enable_video: boolean
  enable_local_fallback: boolean
  enable_ffmpeg: boolean
  ffmpeg_path: string
  request_timeout_seconds: number
  tts_provider: TtsProvider
  tts_voice: string
  tts_model: string
  tts_base_url: string
  tts_api_key: string
  video_width: number
  video_height: number
  video_fps: number
}

export interface TaskRecord {
  id: string
  title: string
  input_file?: string | null
  input_text?: string | null
  status: TaskStatus
  progress: number
  current_step: string
  output_dir?: string | null
  pptx_path?: string | null
  docx_path?: string | null
  script_path?: string | null
  video_path?: string | null
  audio_path?: string | null
  subtitle_path?: string | null
  json_path?: string | null
  log_path?: string | null
  created_at: string
  updated_at: string
  error_message?: string | null
}

export interface CreateTaskRequest {
  title: string
  input_file?: string | null
  input_text: string
  style?: string
  template?: string
  outputs?: string[]
}

export interface LocalFileItem {
  name: string
  path: string
  file_type: string
  size: number
  created_at: string
  previewable: boolean
}

export interface SystemStatusItem {
  name: string
  ok: boolean
  detail: string
}

export interface ProviderTestResult {
  ok: boolean
  provider: string
  message: string
}

export interface VoiceOption {
  id: string
  label: string
  provider: string
  detail: string
}

declare global {
  interface Window {
    __TAURI_INTERNALS__?: {
      invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T>
    }
  }
}

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const api = window.__TAURI_INTERNALS__
  if (!api?.invoke) {
    throw new Error('当前页面未运行在 Tauri 桌面环境中')
  }
  return api.invoke<T>(cmd, args)
}

export const api = {
  getConfig: () => invoke<AppConfig>('get_app_config'),
  saveConfig: (config: AppConfig) => invoke<AppConfig>('save_app_config', { config }),
  resetConfig: () => invoke<AppConfig>('reset_app_config'),
  listTtsVoices: () => invoke<VoiceOption[]>('list_tts_voices'),
  testAi: (config?: AppConfig) => invoke<ProviderTestResult>('test_ai_connection', { config }),
  createTask: (request: CreateTaskRequest) => invoke<TaskRecord>('create_task', { request }),
  listTasks: (search?: string, status?: string) => invoke<TaskRecord[]>('list_tasks', { search, status }),
  getTask: (id: string) => invoke<TaskRecord>('get_task', { id }),
  deleteTask: (id: string) => invoke<void>('delete_task', { id }),
  rerunTask: (id: string) => invoke<TaskRecord>('rerun_task', { id }),
  scanFiles: () => invoke<LocalFileItem[]>('scan_output_files'),
  previewFile: (path: string) => invoke<string>('preview_file', { path }),
  openPath: (path: string) => invoke<void>('open_path', { path }),
  openInFolder: (path: string) => invoke<void>('open_in_folder', { path }),
  systemStatus: () => invoke<SystemStatusItem[]>('get_system_status')
}

export function emptyConfig(): AppConfig {
  return {
    ai_provider: 'local',
    api_base_url: 'https://api.openai.com/v1',
    api_key: '',
    model_name: 'gpt-4o-mini',
    ollama_url: 'http://127.0.0.1:11434',
    output_dir: '',
    enable_tts: true,
    enable_video: true,
    enable_local_fallback: true,
    enable_ffmpeg: true,
    ffmpeg_path: '',
    request_timeout_seconds: 60,
    tts_provider: 'windows_sapi',
    tts_voice: 'windows_default',
    tts_model: 'tts-1',
    tts_base_url: 'http://127.0.0.1:8080/v1',
    tts_api_key: '',
    video_width: 1920,
    video_height: 1080,
    video_fps: 24
  }
}
