import { useMemo } from 'react'
import { useLocale } from './i18n'
import type { Segment } from './types'
import { timestamp } from './api'

export function CueReview({ segments, active, open }: { segments: Segment[]; active?: Segment; open: (segment: Segment) => void }) {
  const { locale } = useLocale()
  const spanish = locale === 'es'
  const issues = useMemo(() => {
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
  return <aside className="panel panel-padding studio-review" aria-label={spanish ? 'Revisión de subtítulos' : 'Caption review'}>
    <h2>{spanish ? 'Revisión de subtítulos' : 'Caption review'}</h2>
    {active?.text && <div className="studio-active-cue">
      <div><span>{spanish ? 'Subtítulo en reproducción' : 'Current caption'}</span> <time>{timestamp(active.start_ms, true)}</time></div>
      <p>{active.speaker ? active.speaker + ': ' + active.text : active.text}</p>
    </div>}
    <p className="hint">{spanish ? 'Velocidad de lectura superior a 20 caracteres por segundo, tiempos inválidos y solapamientos.' : 'Reading speed over 20 characters per second, invalid times and overlaps.'}</p>
    {!issues.length ? <p className="good">{spanish ? 'Sin incidencias detectadas' : 'No issues detected'}</p> :
      <details open>
        <summary>{issues.length} {spanish ? 'segmentos para revisar' : 'segments to review'}</summary>
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
