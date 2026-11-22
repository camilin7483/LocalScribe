import { useEffect, useMemo, useRef } from 'react'
import { useLocale } from './i18n'
import type { Format, Segment } from './types'
import { timestamp } from './api'

export function useCueAudit(segments: Segment[]) {
  return useMemo(() => {
    let previousEnd = 0
    return segments.flatMap((segment) => {
      const duration = (segment.end_ms - segment.start_ms) / 1000
      const caption = segment.speaker ? segment.speaker + ': ' + segment.text.trim() : segment.text.trim()
      const cps = duration > 0 ? Array.from(caption).length / duration : null
      const overlap = segment.start_ms < previousEnd
      if (segment.text.trim()) previousEnd = Math.max(previousEnd, segment.end_ms)
      if (!segment.text.trim() || (!overlap && cps !== null && cps <= 20)) return []
      return [{ segment, cps, overlap }]
    })
  }, [segments])
}

type CueIssue = ReturnType<typeof useCueAudit>[number]

export function CueReview({ issues, active, open }: { issues: CueIssue[]; active?: Segment; open: (segment: Segment) => void }) {
  const { locale } = useLocale()
  const spanish = locale === 'es'
  return <aside className="panel panel-padding studio-review" aria-label={spanish ? 'Revisión de subtítulos' : 'Caption review'}>
    <h2>{spanish ? 'Revisión de subtítulos' : 'Caption review'}</h2>
    {active?.text && <div className="studio-active-cue">
      <div><span>{spanish ? 'Subtítulo en reproducción' : 'Current caption'}</span> <time>{timestamp(active.start_ms, true)}</time></div>
      <p>{active.speaker ? active.speaker + ': ' + active.text : active.text}</p>
    </div>}
    <p className="hint">{spanish ? 'Velocidad de lectura superior a 20 caracteres por segundo, tiempos inválidos y solapamientos.' : 'Reading speed over 20 characters per second, invalid times and overlaps.'}</p>
    {!issues.length ? <p className="good">{spanish ? 'Sin incidencias detectadas' : 'No issues detected'}</p> :
      <details open>
        <summary>{issues.length} {issues.length === 1 ? (spanish ? 'segmento para revisar' : 'segment to review') : (spanish ? 'segmentos para revisar' : 'segments to review')}</summary>
        <ul>{issues.slice(0, 60).map(({ segment, cps, overlap }) => (
          <li key={segment.id}><button className="button small secondary" onClick={() => open(segment)}>
            <time>{timestamp(segment.start_ms, true)}</time>
            {cps === null ? (spanish ? 'Tiempo inválido' : 'Invalid time') : cps.toLocaleString(locale, { maximumFractionDigits: 1 }) + ' CPS'}
            {overlap && (spanish ? ' · Solapamiento' : ' · Overlap')}
          </button></li>
        ))}</ul>
      </details>}
  </aside>
}

export function CaptionExportCheck({ format, issues, close, review, proceed }: {
  format: Format
  issues: CueIssue[]
  close: () => void
  review: () => void
  proceed: () => void
}) {
  const { locale, t } = useLocale()
  const spanish = locale === 'es'
  const dialog = useRef<HTMLDialogElement | null>(null)
  useEffect(() => {
    const node = dialog.current
    node?.showModal()
    return () => { node?.close() }
  }, [])
  const overlaps = issues.filter((issue) => issue.overlap).length
  const invalid = issues.filter((issue) => issue.cps === null).length
  const fast = issues.filter((issue) => issue.cps !== null && issue.cps > 20).length

  return <dialog ref={dialog} className="caption-export-check"
    aria-labelledby="caption-check-heading" onCancel={(event) => { event.preventDefault(); close() }}>
    <h2 id="caption-check-heading">{spanish ? 'Revisar antes de exportar' : 'Review before export'}</h2>
    <ul>
      {!!fast && <li>{fast} {spanish ? 'con lectura por encima de 20 caracteres por segundo' : 'over 20 characters per second'}</li>}
      {!!overlaps && <li>{overlaps} {spanish ? 'con solapamiento' : 'with overlapping times'}</li>}
      {!!invalid && <li>{invalid} {spanish ? 'con tiempos inválidos' : 'with invalid times'}</li>}
    </ul>
    <div className="cue-timing-actions">
      <button className="button small primary" autoFocus onClick={review}>{spanish ? 'Ir al primer segmento' : 'Go to first caption'}</button>
      <button className="button small secondary" onClick={proceed}>{t('export')} {format.toUpperCase()}</button>
      <button className="button small secondary" onClick={close}>{t('cancel')}</button>
    </div>
  </dialog>
}
