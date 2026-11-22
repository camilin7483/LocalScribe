import { ArrowRight, FileAudio, FileVideo, Search, Trash2 } from 'lucide-react'
import { useCallback, useEffect, useState } from 'react'
import { bytes, errorText, live, request, timestamp } from './api'
import { useLocale } from './i18n'
import type { Job, Jobs as JobPage } from './types'
import { FilesEmpty, JobProgress, PageTitle, StatusPill } from './ui'

export function useJobs(view = '', keepPolling = true) {
  const [jobs, setJobs] = useState<Job[]>([])
  const [cursor, setCursor] = useState<number | null>(null)
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(true)
  const refresh = useCallback(async (before?: number) => {
    try {
      const data = await request<JobPage>('/jobs?' + new URLSearchParams({ view, ...(before ? { before: String(before) } : {}) }))
      setJobs((old) => before ? [...new Map([...old, ...data.jobs].map((job) => [job.id, job])).values()] : [
        ...data.jobs, ...(data.jobs.length === 60 ? old.filter((job) => job.created_at <= data.jobs[59].created_at && !data.jobs.some((recent) => recent.id === job.id)) : []),
      ])
      setCursor((old) => before || old === null ? data.cursor : old)
      setError('')
    } catch (err) { setError(errorText(err)) }
    finally { setLoading(false) }
  }, [view])
  useEffect(() => { void refresh() }, [refresh])
  const hasLive = jobs.some(live)
  useEffect(() => {
    if (!hasLive || !keepPolling) return
    const timer = window.setInterval(() => { void refresh() }, 475)
    return () => window.clearInterval(timer)
  }, [hasLive, keepPolling, refresh])
  return { jobs, error, loading, refresh, cursor }
}

function MediaIcon({ name }: { name: string }) {
  const video = /\.(mp4|mkv|mov|webm|avi)$/i.test(name)
  return <span className={'media-icon' + (video ? ' video-icon' : '')}>{video ? <FileVideo size={19}/> : <FileAudio size={19}/>}</span>
}

export function RecentRows({ jobs, loading }: { jobs: Job[]; loading: boolean }) {
  const { t, locale } = useLocale()
  if (loading && !jobs.length) return <div className="empty" role="status">{t('loading')}</div>
  if (!jobs.length) return <FilesEmpty text={t('emptyRecent')}/>
  return <ul className="recent-list">{jobs.slice(0, 4).map((job) => <li key={job.id}>
    <a href={'#editor/' + job.id}>
      <MediaIcon name={job.name}/><span className="job-name"><strong>{job.name}</strong>
        <small>{job.duration_ms ? timestamp(job.duration_ms) : bytes(job.bytes, locale)}</small></span>
      <StatusPill job={job}/><ArrowRight className="row-arrow" size={16}/>
    </a>
  </li>)}</ul>
}

type ListView = 'library' | 'queue' | 'history' | 'benchmarks'
export function JobsView({ view, changed }: { view: ListView; changed: () => void }) {
  const { t, locale } = useLocale()
  const { jobs, error, loading, refresh, cursor } = useJobs(view)
  const [query, setQuery] = useState('')
  const [actionError, setActionError] = useState('')
  const [pending, setPending] = useState<string | null>(null)
  const benchmark = view === 'benchmarks'
  const descriptions = { library: 'libraryDesc', queue: 'queueDesc', history: 'historyDesc', benchmarks: 'benchmarkDesc' } as const
  const filtered = jobs.filter((job) => job.name.toLocaleLowerCase(locale).includes(query.trim().toLocaleLowerCase(locale)))

  async function act(job: Job, action: 'cancel' | 'resume' | 'remove') {
    if (action === 'remove' && !window.confirm(t('removeConfirm'))) return
    setPending(job.id)
    setActionError('')
    try {
      await request('/jobs/' + job.id + (action === 'remove' ? '' : '/' + action), { method: action === 'remove' ? 'DELETE' : 'POST' })
      await refresh()
      changed()
    } catch (err) { setActionError(errorText(err)); void refresh() }
    finally { setPending(null) }
  }

  return <>
    <PageTitle title={t(view)} description={t(descriptions[view])} action={
      <a className="button primary" href={benchmark ? '#new/benchmark' : '#new'}>{t(benchmark ? 'benchmarkNew' : 'newFiles')}<ArrowRight size={17}/></a>
    }/>
    <section className="panel">
      <div className="list-tools"><label className="search"><Search size={17}/><input type="search" value={query} onChange={(event) => setQuery(event.target.value)} placeholder={t('search')} aria-label={t('search')}/></label>
        <button className="button small secondary" onClick={() => void refresh()}>{t('refresh')}</button></div>
      {(error || actionError) && <p className="notice error" role="alert">{actionError || error}</p>}
      {loading && !jobs.length ? <div className="empty" role="status">{t('loading')}</div> : !filtered.length ? <FilesEmpty text={t('noJobs')}/> :
      <ul className="job-list">{filtered.map((job) => <li key={job.id} className="job-item" data-testid={'job-' + job.id}>
        <div className="job-main"><MediaIcon name={job.name}/>
          <div className="job-name"><a href={'#editor/' + job.id}>{job.name}</a><small>
            {bytes(job.bytes, locale)} · {new Date(job.created_at).toLocaleDateString(locale)}{job.detected_language ? ' · ' + job.detected_language.toUpperCase() : ''}
          </small></div><StatusPill job={job}/></div>
        <div className="job-detail">{(view === 'queue' || live(job)) && <JobProgress job={job}/>}
          {job.error && <span className="inline-error">{job.error}</span>}
          {benchmark && job.status === 'completed' && <JobMetrics job={job} />}</div>
        <div className="row-actions">
          {!benchmark && !live(job) && job.status === 'completed' && <a className="button small secondary" href={'#editor/' + job.id}>{t('open')}<ArrowRight size={14}/></a>}
          {live(job) && <button className="button small secondary" disabled={pending !== null} onClick={() => void act(job, 'cancel')}>{t('cancel')}</button>}
          {!live(job) && job.status !== 'completed' && <button className="button small secondary" disabled={pending !== null} onClick={() => void act(job, 'resume')}>{t('resume')}</button>}
          {!live(job) && <button className="icon-button danger" disabled={pending !== null} onClick={() => void act(job, 'remove')} aria-label={t('remove')} title={t('remove')}><Trash2 size={17}/></button>}
        </div>
      </li>)}</ul>}
      {cursor !== null && <div className="panel-footer"><button className="button secondary" onClick={() => void refresh(cursor)}>{t('more')}</button></div>}
    </section>
    {benchmark && <p className="hint mt-4">{t('statsHint')}</p>}
  </>
}

function JobMetrics({ job }: { job: Job }) {
  const { t, locale } = useLocale()
  const number = (value: number, digits = 2) => new Intl.NumberFormat(locale, { maximumFractionDigits: digits }).format(value)
  const m = job.metrics
  const spanish = locale === 'es'
  return <><dl className="benchmark-metrics">
    <div><dt>{t('model')}</dt><dd>{job.plan.model}</dd></div>
    <div><dt>{t('runtime')}</dt><dd>{number(m.wall_ms / 1000)} s</dd></div>
    <div><dt>{t('rtf')}</dt><dd>{number(m.wall_ms / (job.duration_ms || 1), 3)}×</dd></div>
    <div><dt>{t('rss')}</dt><dd>{m.peak_rss_bytes ? bytes(m.peak_rss_bytes, locale) : t('none')}</dd></div>
    <div><dt>{t('cpuUsage')}</dt><dd>{m.cpu_ms ? number(100 * m.cpu_ms / m.wall_ms, 1) + '%' : t('none')}</dd></div>
    <div><dt>{t('wer')}</dt><dd>{m.wer === null ? t('none') : number(m.wer * 100, 1) + '%'}</dd></div>
  </dl>
    {m.stages && <details className="benchmark-stages">
      <summary>{spanish ? 'Desglose del procesamiento' : 'Processing breakdown'}</summary>
      <dl className="benchmark-metrics">
        <div><dt>{spanish ? 'Inspección' : 'Probe'}</dt><dd>{number(m.stages.probe_ms)} ms</dd></div>
        <div><dt>{spanish ? 'Decodificación' : 'Decode'}</dt><dd>{number(m.stages.decode_ms)} ms</dd></div>
        <div><dt>{spanish ? 'Inferencia' : 'Inference'}</dt><dd>{number(m.stages.inference_ms)} ms</dd></div>
        <div><dt>{spanish ? 'Ventanas sin señal' : 'Silent windows'}</dt><dd>{number(m.stages.silent_windows)}</dd></div>
        <div><dt>{spanish ? 'Ventanas persistentes' : 'Persistent windows'}</dt><dd>{number(m.stages.persistent_windows)}</dd></div>
        <div><dt>{spanish ? 'Pico estimado de procesos' : 'Estimated process peak'}</dt><dd>{bytes(m.stages.peak_child_tree_estimate_bytes, locale)}</dd></div>
      </dl>
      {!!m.stages.worker_fallbacks && <p>{spanish ? 'Reintentos con el motor estándar' : 'Retries with standard engine'} · {m.stages.worker_fallbacks}</p>}
    </details>}
  </>
}

export function PickTranscript({ value, onPick }: { value: string; onPick: (id: string) => void }) {
  const { t } = useLocale()
  const { jobs, loading, cursor, refresh, error } = useJobs('history', false)
  const completed = jobs.filter((job) => job.status === 'completed')
  return <><div className="transcript-select"><label className="field"><span>{t('pickTranscript')}</span>
    <select aria-label={t('pickTranscript')} value={value} onChange={(event) => onPick(event.target.value)}>
      <option value="">{loading ? t('loading') : t('pickTranscript')}</option>
      {completed.map((job) => <option value={job.id} key={job.id}>{job.name}</option>)}
    </select></label>{cursor !== null && <button className="button secondary" onClick={() => void refresh(cursor)}>{t('more')}</button>}</div>
    {error && <p className="notice error" role="alert">{error}</p>}
    {!loading && !completed.length && <FilesEmpty text={t('noTranscript')} action={<a className="button secondary" href="#new">{t('new')}</a>}/>}
  </>
}
