import { ArrowRight, Check, Sparkles } from 'lucide-react'
import { useEffect, useState } from 'react'
import { errorText, request, timestamp } from './api'
import { useDetail } from './editor'
import { useLocale } from './i18n'
import { PickTranscript } from './jobs'
import type { AnalyzeMode, Engines, Insight, System } from './types'
import { PageTitle } from './ui'

const modes: AnalyzeMode[] = ['general', 'executive', 'key_points', 'decisions', 'tasks', 'questions', 'translate', 'correct', 'ask', 'speakers']

export function AnalysisView({ system }: { system: System }) {
  const { t, locale, languageName } = useLocale()
  const [engines, setEngines] = useState<Engines | null>(null)
  const [id, setId] = useState('')
  const [model, setModel] = useState('')
  const [mode, setMode] = useState<AnalyzeMode>('general')
  const [target, setTarget] = useState(locale === 'es' ? 'en' : 'es')
  const [question, setQuestion] = useState('')
  const [speakerCount, setSpeakerCount] = useState('')
  const [reload, setReload] = useState(0)
  const [result, setResult] = useState<Insight | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [applied, setApplied] = useState(false)
  const [resultRevision, setResultRevision] = useState(0)
  const { detail } = useDetail(id, reload)
  useEffect(() => {
    let ignore = false
    void request<Engines>('/engines').then((engines) => {
      if (!ignore) { setEngines(engines); setModel(engines.models[0] ?? '') }
    }).catch((err) => { if (!ignore) setError(errorText(err)) })
    return () => { ignore = true }
  }, [system.settings.ollama_enabled])

  const canRun = mode === 'speakers' ? engines?.diarization : !!model
  async function run() {
    if (!detail) return
    setBusy(true); setError(''); setResult(null); setApplied(false)
    try {
      const result = await request<Insight>('/jobs/' + id + '/analyze', {
        method: 'POST', body: JSON.stringify({
          mode, model, target, question, locale, speaker_count: mode === 'speakers' && speakerCount ? Number(speakerCount) : null, revision: detail.job.revision,
        }),
      })
      setResultRevision(detail.job.revision)
      setResult(result)
    } catch (err) { setError(errorText(err)) }
    finally { setBusy(false) }
  }

  async function apply() {
    if (!result?.changes.length || !detail || applied) return
    setError(''); setBusy(true)
    try {
      await request('/jobs/' + id + '/suggestions', {
        method: 'PATCH', body: JSON.stringify({ revision: resultRevision, changes: result.changes }),
      })
      setApplied(true)
      setReload((value) => value + 1)
    } catch (err) { setError(errorText(err)) }
    finally { setBusy(false) }
  }

  return <>
    <PageTitle title={t('analysis')} description={t('analysisDesc')}/>
    <div className="work-grid">
      <div className="stack">
        <section className="panel panel-padding form-stack">
          <PickTranscript value={id} onPick={(value) => { if (!busy) { setId(value); setResult(null) } }}/>
          <label className="field"><span>{t('analysisMode')}</span><select value={mode} disabled={busy} onChange={(event) => { setMode(event.target.value as AnalyzeMode); setResult(null) }}>
            {modes.map((item) => <option key={item} value={item}>{t(item)}</option>)}
          </select></label>
          {mode !== 'speakers' && !!engines?.models.length && <label className="field"><span>{t('aiReady')}</span><select value={model} disabled={busy} onChange={(event) => setModel(event.target.value)}>
            {engines.models.map((model) => <option key={model}>{model}</option>)}
          </select></label>}
          {mode === 'translate' && <label className="field"><span>{t('target')}</span><select value={target} disabled={busy} onChange={(event) => setTarget(event.target.value)}>
            {['en', 'es', 'pt', 'fr', 'de', 'it', 'zh', 'ja', 'ko', 'ar'].map((code) => <option key={code} value={code}>{languageName(code)}</option>)}
          </select></label>}
          {mode === 'ask' && <label className="field"><span>{t('ask')}</span><textarea value={question} disabled={busy} maxLength={800} rows={3} placeholder={t('askPlaceholder')} onChange={(event) => setQuestion(event.target.value)}/></label>}
          {mode === 'speakers' && <label className="field"><span>{t('speakerCount')}</span><input type="number" min={2} max={12}
            placeholder={t('auto')} value={speakerCount} disabled={busy} onChange={(event) => setSpeakerCount(event.target.value)}/></label>}
          <button className="button primary justify-center" disabled={!id || !detail || !canRun || busy || (mode === 'ask' && !question.trim())} onClick={() => void run()}>
            <Sparkles size={17}/>{busy ? t('analyzing') : t('run')}</button>
        </section>
        {error && <p className="notice error" role="alert">{error}</p>}
        {result && <section className="panel panel-padding insight" data-testid="analysis-result">
          <h2>{result.title || t('result')}</h2>
          {result.text && <p className="insight-text">{result.text}</p>}
          {!!result.evidence.length && <><h3>{t('evidence')}</h3><ol>{result.evidence.map((segment) => <li key={segment.id}><a className="mono" href={'#editor/' + id}>{timestamp(segment.start_ms, true)}</a><q>{segment.text}</q></li>)}</ol></>}
          {!!result.changes.length && <><h3>{t('changes')}</h3><ol>{result.changes.map((change) => <li key={change.id}>
            <span className="mono">{timestamp(detail?.segments.find((segment) => segment.id === change.id)?.start_ms ?? 0)}</span>
            <span>{change.speaker && <strong>{change.speaker} · </strong>}{change.text}
            {!change.speaker && <details><summary>{t('original')}</summary>{detail?.segments.find((segment) => segment.id === change.id)?.original}</details>}
            </span></li>)}</ol>
            <button className="button secondary" disabled={applied || busy} onClick={() => void apply()}><Check size={16}/>{t(applied ? 'saved' : 'apply')}</button></>}
        </section>}
      </div>
      <section className="panel panel-padding self-start stack">
        <div className="inline-heading"><Sparkles size={19}/><h2>{t('engines')}</h2></div>
        <p className="hint">{!engines ? t('loading') : mode === 'speakers' ? (engines.diarization ? 'sherpa-onnx · CPU' : t('optionalSpeakers')) :
          !engines.enabled ? t('aiDisabled') : engines.models.length ? t('aiReady') + ' · ' + model : t('aiUnavailable')}</p>
        {engines && !engines.enabled && <a className="button small secondary" href="#privacy">{t('privacy')}<ArrowRight size={15}/></a>}
      </section>
    </div>
  </>
}
