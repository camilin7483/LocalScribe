import type { Format, Job, Options } from './types'

export class RequestError extends Error {
  constructor(message: string, readonly status: number) { super(message) }
}

export async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers)
  headers.set('X-LocalScribe', '1')
  if (init.body) headers.set('Content-Type', 'application/json')
  const response = await fetch('/api' + path, { ...init, headers, cache: 'no-store', mode: 'same-origin' })
  if (!response.ok) {
    const body = await response.json().catch(() => ({}))
    throw new RequestError(body.error ?? 'LocalScribe could not complete the request.', response.status)
  }
  if (response.status === 204 || response.status === 202) return undefined as T
  return response.json()
}

export function upload(file: File, options: Options, update: (progress: number | null) => void): Promise<Job> {
  return new Promise((resolve, reject) => {
    const form = new FormData()
    form.append('options', JSON.stringify(options))
    form.append('file', file)
    const xhr = new XMLHttpRequest()
    xhr.open('POST', '/api/jobs')
    xhr.setRequestHeader('X-LocalScribe', '1')
    xhr.responseType = 'json'
    xhr.upload.onprogress = (event) => update(event.lengthComputable ? event.loaded / event.total : null)
    xhr.onload = () => xhr.status === 202 ? resolve(xhr.response) :
      reject(new RequestError(xhr.response?.error ?? 'Could not import this file.', xhr.status))
    xhr.onerror = () => reject(new Error('Connection to LocalScribe was lost.'))
    xhr.send(form)
  })
}

export async function exportJob(job: Job, format: Format) {
  const response = await fetch('/api/jobs/' + job.id + '/export/' + format, { headers: { 'X-LocalScribe': '1' } })
  if (!response.ok) throw new Error('Could not export this transcript.')
  const url = URL.createObjectURL(await response.blob())
  const link = document.createElement('a')
  link.href = url
  link.download = job.name.replace(/\.[^.]+$/, '') + '.' + format
  link.click()
  window.setTimeout(() => URL.revokeObjectURL(url), 1000)
}

export function timestamp(ms: number, fine = false) {
  const value = Math.max(0, Math.floor(ms))
  const hours = Math.floor(value / 3_600_000)
  const minutes = Math.floor(value / 60_000) % 60
  const seconds = Math.floor(value / 1000) % 60
  const clock = (hours ? String(hours).padStart(2, '0') + ':' : '') +
    String(minutes).padStart(2, '0') + ':' + String(seconds).padStart(2, '0')
  return fine ? clock + '.' + String(value % 1000).padStart(3, '0') : clock
}

export function bytes(value: number, language = 'en') {
  const unit = ['B', 'KiB', 'MiB', 'GiB']
  const index = Math.max(0, Math.min(3, Math.floor(Math.log2(value || 1) / 10)))
  return new Intl.NumberFormat(language, { maximumFractionDigits: index ? 1 : 0 }).format(value / 1024 ** index) + ' ' + unit[index]
}

export const live = (job: Job) => job.status === 'queued' || job.status === 'running'
export const errorText = (error: unknown) => error instanceof Error ? error.message : String(error)
