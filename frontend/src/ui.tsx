import { AudioLines, ChevronDown, CloudDownload, FileAudio, FolderOpen, ShieldCheck, Upload } from 'lucide-react'
import { useRef, useState, type ReactNode } from 'react'
import { bytes, timestamp } from './api'
import { useLocale, type Key } from './i18n'
import type { Job, Model, Plan } from './types'

export const profiles = ['eco', 'balanced', 'precision'] as const
export const formats = ['srt', 'vtt', 'txt', 'md', 'json'] as const

export function PageTitle({ title, description, action }: { title: string; description?: string; action?: ReactNode }) {
  return <div className="page-title"><div><h1>{title}</h1>{description && <p>{description}</p>}</div>{action}</div>
}

export function Dropzone({ onFiles, compact = false }: { onFiles: (files: File[]) => void; compact?: boolean }) {
  const input = useRef<HTMLInputElement>(null)
  const [drag, setDrag] = useState(false)
  const { t } = useLocale()
  return <div className={'dropzone' + (drag ? ' dragging' : '') + (compact ? ' compact' : '')} data-testid="dropzone"
    onDragOver={(event) => { event.preventDefault(); event.dataTransfer.dropEffect = 'copy'; setDrag(true) }}
    onDragLeave={(event) => { if (!event.currentTarget.contains(event.relatedTarget as Node)) setDrag(false) }}
    onDrop={(event) => { event.preventDefault(); setDrag(false); onFiles(Array.from(event.dataTransfer.files)) }}>
    <input type="file" aria-label={t('browse')} ref={input} multiple className="sr-only" tabIndex={-1}
      accept=".mp3,.wav,.flac,.m4a,.ogg,.aac,.mp4,.mkv,.mov,.webm,.avi" onChange={(event) => {
        onFiles(Array.from(event.target.files ?? [])); event.target.value = ''
      }} />
    <div className="drop-symbol"><Upload size={23} /></div>
    <h2>{t('uploadTitle')}</h2><p>{t('uploadDesc')}</p>
    <button className="button primary" onClick={() => input.current?.click()}><FolderOpen size={17}/>{t('browse')}</button>
    <small>{t('supported')}</small>
  </div>
}

export function FilesEmpty({ text, action }: { text: string; action?: ReactNode }) {
  return <div className="empty"><div className="empty-icon"><FileAudio size={22}/></div><p>{text}</p>{action}</div>
}

export function StatusPill({ job }: { job: Job }) {
  const { t } = useLocale()
  return <span className={'pill status-' + job.status} data-testid={'status-' + job.id}><span aria-hidden="true" />
    {job.status === 'completed' ? (t('completed') === 'Completadas' ? 'Completada' : t('completed')) : t(job.status)}
  </span>
}

export function JobProgress({ job }: { job: Job }) {
  const { t } = useLocale()
  if (!job.duration_ms) return null
  const value = Math.min(job.processed_ms / job.duration_ms, 1)
  return <div className="job-progress"><progress aria-label={t('running')} value={value} max={1}/>
    <span className="mono">{timestamp(job.processed_ms)} / {timestamp(job.duration_ms)} · {Math.floor(value * 100)}%</span>
  </div>
}

export function PlanDetails({ plan, model }: { plan: Plan; model?: Model }) {
  const { t } = useLocale()
  const rows: [Key, string | number][] = [
    ['model', (model?.label ?? plan.model) + ' · ' + plan.quantization], ['engine', plan.engine],
    ['threads', plan.threads], ['window', timestamp(plan.chunk_ms)], ['workers', plan.workers],
    ['memory', plan.memory_mib + ' MiB'],
  ]
  return <dl className="key-values">{rows.map(([label, value]) => <div key={label}><dt>{t(label)}</dt><dd>{value}</dd></div>)}</dl>
}

export function ModelStatus({ model }: { model: Model }) {
  const { t, locale } = useLocale()
  if (model.downloading) return <div className="transfer"><progress value={model.downloaded} max={model.size} aria-label={t('install')} />
    <span>{bytes(model.downloaded, locale)} / {bytes(model.size, locale)}</span></div>
  return <span className={'pill ' + (model.installed ? 'status-completed' : '')}>
    {model.installed ? <ShieldCheck size={13}/> : <CloudDownload size={13}/>}{t(model.installed ? 'installed' : 'absent')}
  </span>
}

export function ExportPicker({ onExport, disabled = false }: { onExport: (format: typeof formats[number]) => void; disabled?: boolean }) {
  const { t } = useLocale()
  return <label className="export-picker button primary"><CloudDownload size={17}/><span>{t('export')}</span><ChevronDown size={14}/>
    <select aria-label={t('export')} value="" disabled={disabled} onChange={(event) => {
      const chosen = formats.find((format) => format === event.target.value)
      if (chosen) onExport(chosen)
    }}><option value="" disabled>{t('export')}</option>{formats.map((format) => <option key={format} value={format}>{format.toUpperCase()}</option>)}</select>
  </label>
}

export function BrandMark() {
  return <span className="brand-mark" aria-hidden="true"><AudioLines size={25} strokeWidth={2.25}/></span>
}
