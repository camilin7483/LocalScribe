import { ArrowLeft, AudioLines, Check, ChevronLeft, ChevronRight, Clock3, Eye, EyeOff, Pause, Play, Redo2, Search, Undo2, UserRound } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { errorText, exportJob, live, request, timestamp } from './api'
import { useEdits } from './editing'
import { useLocale } from './i18n'
import { PickTranscript } from './jobs'
import type { Detail, Format, Segment } from './types'
import { ExportPicker, FilesEmpty, JobProgress, PageTitle, StatusPill } from './ui'

export function useDetail(id: string, reload = 0) {
  const [detail, setDetail] = useState<Detail | null>(null)
  const [error, setError] = useState('')
  useEffect(() => {
    if (!id) return
    let stopped = false
    let timer = 0
    async function fetchDetail() {
      try {
        const next = await request<Detail>('/jobs/' + id)
        if (!stopped) {
          setDetail(next)
          setError('')
          if (live(next.job)) timer = window.setTimeout(() => void fetchDetail(), 450)
        }
      } catch (err) { if (!stopped) setError(errorText(err)) }
    }
    void fetchDetail()
    return () => { stopped = true; window.clearTimeout(timer) }
  }, [id, reload])
  return { detail: detail?.job.id === id ? detail : null, error }
}

export function EditorView({ id }: { id?: string }) {
  const { t } = useLocale()
  if (!id) return <><PageTitle title={t('editor')}/><section className="panel panel-padding">
    <PickTranscript value="" onPick={(jobId) => { window.location.hash = '#editor/' + jobId }}/>
  </section></>
  return <OpenedEditor key={id} id={id}/>
}

function OpenedEditor({ id }: { id: string }) {
  const { t } = useLocale()
  const { detail, error } = useDetail(id)
  if (error) return <p className="notice error" role="alert">{error}</p>
  if (!detail) return <p className="notice" role="status">{t('loading')}</p>
  if (detail.job.status !== 'completed') return <>
    <PageTitle title={detail.job.name} action={<a className="button secondary" href="#queue"><ArrowLeft size={17}/>{t('queue')}</a>}/>
    <section className="panel panel-padding stack"><StatusPill job={detail.job}/><JobProgress job={detail.job}/>
      {detail.job.error && <p className="inline-error">{detail.job.error}</p>}
      {!detail.segments.length && !live(detail.job) && <p>{t('noSpeech')}</p>}
      {detail.segments.map((segment) => <div key={segment.id} className="partial-line"><time className="mono">{timestamp(segment.start_ms)}</time><p>{segment.text}</p></div>)}
      {!!detail.segments.length && <ExportPicker onExport={(format) => { void exportJob(detail.job, format).catch((err) => window.alert(errorText(err))) }}/>}
    </section>
  </>
  return <EditableTranscript key={detail.job.id} detail={detail}/>
}

function EditableTranscript({ detail }: { detail: Detail }) {
  const { t, locale, languageName } = useLocale()
  const edit = useEdits(detail)
  const { job } = detail
  const [query, setQuery] = useState('')
  const [showOriginal, setShowOriginal] = useState(false)
  const [position, setPosition] = useState(0)
  const [selected, setSelected] = useState<number | null>(null)
  const [flash, setFlash] = useState('')
  const [playerError, setPlayerError] = useState('')
  const [playing, setPlaying] = useState(false)
  const [mediaDuration, setMediaDuration] = useState(job.duration_ms ?? 0)
  const [preview, setPreview] = useState(detail.preview_ready)
  const [preparing, setPreparing] = useState(false)
  const player = useRef<HTMLMediaElement | null>(null)
  const current = edit.segments.find((segment) => position >= segment.start_ms && position < segment.end_ms)?.id ?? null
  const term = query.trim().toLocaleLowerCase(locale)
  const shown = edit.segments.filter((segment) => !term || [segment.text, segment.speaker ?? '', showOriginal ? segment.original : ''].some(
    (value) => value.toLocaleLowerCase(locale).includes(term)))
  const mediaSource = preview ? 'preview' : detail.original_playback ? 'original' : ''
  const source = mediaSource ? '/api/media/' + job.id + '/' + mediaSource : ''
  const isVideo = detail.video && !preview

  function seek(value: number, segment?: Segment) {
    if (player.current && player.current.readyState >= 1) player.current.currentTime = value / 1000
    setPosition(value)
    if (segment) {
      setSelected(segment.id)
      document.getElementById('segment-' + segment.id)?.scrollIntoView({ block: 'nearest', behavior: window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 'instant' : 'smooth' })
    }
  }

  function jump(direction: -1 | 1) {
    const index = shown.findIndex((segment) => segment.id === (selected ?? current))
    const next = shown[Math.max(0, Math.min(shown.length - 1, index + direction))]
    if (next) seek(next.start_ms, next)
  }

  async function play() {
    const media = player.current
    if (!media) return
    try {
      if (media.paused) await media.play()
      else media.pause()
    } catch (err) { setPlayerError(errorText(err)) }
  }

  async function prepare() {
    setPreparing(true); setPlayerError('')
    try { await request('/jobs/' + job.id + '/preview', { method: 'POST' }); setPreview(true) }
    catch (error) { setPlayerError(errorText(error)) }
    finally { setPreparing(false) }
  }

  async function download(format: Format) {
    setFlash('')
    if (!await edit.save()) return
    try { await exportJob(job, format); setFlash(t('exported') + ' · ' + format.toUpperCase()) }
    catch (err) { setFlash(errorText(err)) }
  }

  const playbackProps = {
    src: source, preload: 'metadata' as const, 'aria-label': t('playback'),
    onTimeUpdate: () => setPosition((player.current?.currentTime ?? 0) * 1000),
    onLoadedMetadata: () => {
      const length = player.current?.duration
      if (length && Number.isFinite(length)) setMediaDuration(length * 1000)
    },
    onPlay: () => setPlaying(true), onPause: () => setPlaying(false), onEnded: () => setPlaying(false),
    onError: () => setPlayerError(locale === 'es' ? 'El navegador no pudo reproducir el archivo.' : 'The browser could not play this file.'),
  }

  return <div className="editor-layout">
    <PageTitle title={job.name} description={[job.detected_language && languageName(job.detected_language), timestamp(job.duration_ms ?? 0), edit.segments.length + ' ' + t('segments')].filter(Boolean).join(' · ')}
      action={<ExportPicker onExport={(format) => void download(format)}/>}/>
    <section className="panel player-panel" aria-label={t('playback')}>
      {source && (isVideo ? <video className="video-player" {...playbackProps} controls ref={(node) => { player.current = node }}/> :
        <audio {...playbackProps} ref={(node) => { player.current = node }}/>)}
      <div className="transport">
        <button className="play-button" onClick={() => void play()} disabled={!source || !!playerError} aria-label={playing ? (locale === 'es' ? 'Pausar' : 'Pause') : (locale === 'es' ? 'Reproducir' : 'Play')}>
          {playing ? <Pause size={19} fill="currentColor"/> : <Play size={19} fill="currentColor"/>}</button>
        <div className="timeline">
          <div className="timeline-label"><span className="mono">{timestamp(position, true)}</span><span className="mono">{timestamp(mediaDuration)}</span></div>
          <input type="range" aria-label={t('time')} min={0} max={Math.max(1, mediaDuration)} step={1} value={position} onChange={(event) => seek(Number(event.target.value))}/>
          <div className="timed-segments" aria-hidden="true">{edit.segments.map((segment) => <span key={segment.id} className={current === segment.id ? 'highlighted' : ''}
            style={{ left: 100 * segment.start_ms / Math.max(1, mediaDuration) + '%', width: 100 * (segment.end_ms - segment.start_ms) / Math.max(1, mediaDuration) + '%' }}/>)}</div>
        </div><AudioLines className="transport-icon" size={21}/>
      </div>
      {(!source || playerError) && <div className="preview-note">
        {playerError && <span role="alert" className="inline-error">{playerError}</span>}
        {!preview && <button className="button secondary" disabled={preparing} onClick={() => void prepare()}>{preparing ? t('preparing') : t('prepare')}</button>}
      </div>}
    </section>
    <section className="panel transcript-panel">
      <div className="editor-tools">
        <label className="search"><Search size={17}/><input type="search" value={query} placeholder={t('segmentSearch')} aria-label={t('segmentSearch')} onChange={(event) => setQuery(event.target.value)}/></label>
        <div className="tool-buttons">
          <button className="icon-button" aria-label={t('before')} title={t('before')} disabled={!shown.length} onClick={() => jump(-1)}><ChevronLeft size={18}/></button>
          <button className="icon-button" aria-label={t('next')} title={t('next')} disabled={!shown.length} onClick={() => jump(1)}><ChevronRight size={18}/></button>
          <span className="tool-divider"/>
          <button className="icon-button" aria-label={t('undo')} title={t('undo')} disabled={!edit.canUndo} onClick={edit.undo}><Undo2 size={18}/></button>
          <button className="icon-button" aria-label={t('redo')} title={t('redo')} disabled={!edit.canRedo} onClick={edit.redo}><Redo2 size={18}/></button>
          <button className="button small secondary" aria-pressed={showOriginal} onClick={() => setShowOriginal(!showOriginal)}>{showOriginal ? <EyeOff size={16}/> : <Eye size={16}/>}{t(showOriginal ? 'hideOriginal' : 'original')}</button>
        </div>
      </div>
      {flash && <p className="notice editor-notice" role="status">{flash}</p>}
      {edit.error && <p className="notice error editor-notice" role="alert">{edit.error}</p>}
      {!shown.length ? <FilesEmpty text={edit.segments.length ? t('noJobs') : t('noSpeech')}/> : <ol className="segment-list">
        {shown.map((segment, index) => <li key={segment.id} id={'segment-' + segment.id} className={(current === segment.id || selected === segment.id) ? 'active-segment' : ''}>
          <button className="cue-time mono" onClick={() => seek(segment.start_ms, segment)}><Clock3 size={14}/>{timestamp(segment.start_ms, true)}</button>
          <div className="segment-body"><label className="speaker-input"><UserRound size={14}/><input aria-label={t('speaker') + ' ' + (index + 1)}
            placeholder={t('speakerPlaceholder')} value={segment.speaker ?? ''} maxLength={60} onChange={(event) => edit.change(segment.id, { speaker: event.target.value || null })}/></label>
            <textarea data-testid={'segment-input-' + segment.id} aria-label={t('segment') + ' ' + (index + 1)} rows={Math.min(5, Math.max(2, Math.ceil(segment.text.length / 95)))}
              value={segment.text} maxLength={8000} onChange={(event) => edit.change(segment.id, { text: event.target.value })}/>
            {showOriginal && <blockquote>{segment.original}</blockquote>}</div>
        </li>)}
      </ol>}
      <div className="editor-footer" aria-live="polite"><span>{edit.state === 'saved' && <Check size={15}/>} {t(edit.state)}</span>
        {edit.state === 'unsaved' && <button className="button small secondary" onClick={() => void edit.save()}>{t('save')}</button>}</div>
    </section>
  </div>
}

export function SubtitlesView() {
  const { t } = useLocale()
  const [selected, setSelected] = useState('')
  const { detail, error } = useDetail(selected)
  const [flash, setFlash] = useState('')
  const stamp = (ms: number) => (ms < 3_600_000 ? '00:' : '') + timestamp(ms, true).replace('.', ',')
  async function download(format: Format) {
    if (!detail) return
    try { await exportJob(detail.job, format); setFlash(t('exported') + ' · ' + format.toUpperCase()) }
    catch (err) { setFlash(errorText(err)) }
  }
  return <>
    <PageTitle title={t('subtitles')} description={t('subtitleDesc')} action={detail && <ExportPicker onExport={(format) => void download(format)}/>}/>
    <section className="panel panel-padding"><PickTranscript value={selected} onPick={(id) => { setSelected(id); setFlash('') }}/></section>
    {error && <p className="notice error mt-4" role="alert">{error}</p>}
    {flash && <p className="notice mt-4" role="status">{flash}</p>}
    {detail && <section className="panel panel-padding mt-5"><div className="panel-heading plain"><h2>SRT · {detail.segments.length} {t('segments')}</h2>
      <a className="button small secondary" href={'#editor/' + detail.job.id}>{t('editor')}</a></div>
      <pre className="caption-preview">{detail.segments.filter((segment) => segment.text.trim()).map((segment, index) =>
        (index + 1) + '\n' + stamp(segment.start_ms) + ' --> ' + stamp(segment.end_ms) + '\n' + (segment.speaker ? segment.speaker + ': ' : '') + segment.text).join('\n\n')}</pre>
    </section>}
  </>
}
