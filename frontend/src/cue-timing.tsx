import { useState } from 'react'
import { timestamp } from './api'
import { useLocale } from './i18n'
import type { Segment } from './types'

const fullTimestamp = (ms: number) => (ms < 3_600_000 ? '00:' : '') + timestamp(ms, true)

function parseTimestamp(value: string) {
  const match = /^(?:(\d+):)?([0-5]?\d):([0-5]\d)(?:[.,](\d{1,3}))?$/.exec(value.trim())
  if (!match) return null
  const [, hours = '0', minutes, seconds, fraction = ''] = match
  const result = (Number(hours) * 3600 + Number(minutes) * 60 + Number(seconds)) * 1000 + Number(fraction.padEnd(3, '0'))
  return Number.isSafeInteger(result) ? result : null
}

export function CueTiming({ segment, duration, position, close, apply }: {
  segment: Segment
  duration: number
  position: number
  close: () => void
  apply: (start: number, end: number) => void
}) {
  const { locale, t } = useLocale()
  const spanish = locale === 'es'
  const [start, setStart] = useState(fullTimestamp(segment.start_ms))
  const [end, setEnd] = useState(fullTimestamp(segment.end_ms))
  const [submitted, setSubmitted] = useState(false)
  const startMs = parseTimestamp(start)
  const endMs = parseTimestamp(end)
  const badRange = startMs !== null && endMs !== null && startMs >= endMs
  const exceedsMedia = endMs !== null && endMs > duration
  const id = 'cue-timing-' + segment.id
  const error = startMs === null || endMs === null
    ? (spanish ? 'Usa el formato 00:00:00.000.' : 'Use the format 00:00:00.000.')
    : badRange
      ? (spanish ? 'El final debe ser posterior al inicio.' : 'End must be after start.')
      : exceedsMedia
        ? (spanish ? 'El final supera la duración del archivo.' : 'End exceeds the media duration.')
        : ''
  const restore = () => {
    setStart(fullTimestamp(segment.original_start_ms))
    setEnd(fullTimestamp(segment.original_end_ms))
    setSubmitted(false)
  }
  const hasOriginal = segment.start_ms !== segment.original_start_ms || segment.end_ms !== segment.original_end_ms
  const playhead = Math.min(duration, Math.max(0, Math.round(position)))

  return <form className="cue-timing" id={id} onSubmit={(event) => {
    event.preventDefault()
    setSubmitted(true)
    if (!error && startMs !== null && endMs !== null) apply(startMs, endMs)
  }} onKeyDown={(event) => {
    if (event.key === 'Escape') { event.stopPropagation(); close() }
  }}>
    <div className="cue-timing-fields">
      <label>{spanish ? 'Inicio' : 'Start'}
        <input className="mono" value={start} onChange={(event) => setStart(event.target.value)} autoFocus
          placeholder="00:00:00.000" aria-invalid={submitted && (startMs === null || badRange)}
          aria-describedby={submitted && error ? id + '-error' : undefined}/>
      </label>
      <label>{spanish ? 'Final' : 'End'}
        <input className="mono" value={end} onChange={(event) => setEnd(event.target.value)}
          placeholder="00:00:00.000" aria-invalid={submitted && (endMs === null || badRange || exceedsMedia)}
          aria-describedby={submitted && error ? id + '-error' : undefined}/>
      </label>
    </div>
    <div className="cue-timing-actions">
      <button type="button" className="button small secondary" onClick={() => setStart(fullTimestamp(playhead))}>
        {spanish ? 'Inicio en posición actual' : 'Start at playhead'}
      </button>
      <button type="button" className="button small secondary" onClick={() => setEnd(fullTimestamp(playhead))}>
        {spanish ? 'Final en posición actual' : 'End at playhead'}
      </button>
      {hasOriginal && <button type="button" className="button small secondary" onClick={restore}>
        {spanish ? 'Recuperar tiempos originales' : 'Restore original timing'}
      </button>}
    </div>
    {submitted && error && <p className="inline-error" id={id + '-error'} role="alert">{error}</p>}
    <div className="cue-timing-actions">
      <button className="button small primary" type="submit">{spanish ? 'Aplicar tiempos' : 'Apply timing'}</button>
      <button type="button" className="button small secondary" onClick={close}>{t('cancel')}</button>
    </div>
  </form>
}
