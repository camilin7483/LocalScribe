import { useEffect, useRef, useState } from 'react'
import { request } from './api'
import { useLocale } from './i18n'

export function Waveform({ id, fraction }: { id: string; fraction: number }) {
  const { locale, theme } = useLocale()
  const [peaks, setPeaks] = useState<number[] | null>(null)
  const [failed, setFailed] = useState(false)
  const [retry, setRetry] = useState(0)
  const canvas = useRef<HTMLCanvasElement>(null)

  useEffect(() => {
    const abort = new AbortController()
    void request<number[]>('/jobs/' + id + '/waveform', { signal: abort.signal })
      .then(setPeaks).catch(() => { if (!abort.signal.aborted) setFailed(true) })
    return () => abort.abort()
  }, [id, retry])

  useEffect(() => {
    const node = canvas.current
    const ctx = node?.getContext('2d')
    if (!peaks || !node || !ctx) return
    const style = getComputedStyle(node)
    const max = peaks.reduce((peak, sample) => Math.max(peak, sample), 1)
    const played = style.getPropertyValue('--mint').trim()
    const upcoming = style.getPropertyValue('--muted').trim()
    ctx.clearRect(0, 0, node.width, node.height)
    peaks.forEach((peak, index) => {
      ctx.fillStyle = index < peaks.length * fraction ? played : upcoming
      const height = Math.max(1, peak / max * (node.height - 6))
      ctx.fillRect(index * 2, (node.height - height) / 2, 1, height)
    })
  }, [peaks, fraction, theme])

  if (failed) return <button className="button small secondary" onClick={() => { setFailed(false); setRetry(retry + 1) }}>
    {locale === 'es' ? 'Cargar forma de onda' : 'Load waveform'}
  </button>
  if (!peaks) return <span className="waveform-placeholder" role="status">
    {locale === 'es' ? 'Analizando señal de audio…' : 'Analyzing audio signal…'}
  </span>
  return <canvas className="studio-waveform" ref={canvas} width={1536} height={76}
    aria-hidden="true" data-testid="studio-waveform" />
}
