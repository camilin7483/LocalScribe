import { ArrowRight, AudioLines, Cpu, FileAudio, Info, Plus, X } from 'lucide-react'
import { useState } from 'react'
import { bytes, errorText, upload } from './api'
import { useLocale } from './i18n'
import type { Options, Profile, System } from './types'
import { Dropzone, PageTitle, PlanDetails, profiles } from './ui'

export function ImportView({ system, entries, setEntries, benchmark, changed }: {
  system: System; entries: File[]; setEntries: (files: File[]) => void; benchmark: boolean; changed: () => void
}) {
  const { t, locale, languageName } = useLocale()
  const [profile, setProfile] = useState<Profile>('balanced')
  const [language, setLanguage] = useState('auto')
  const [model, setModel] = useState('')
  const [vad, setVad] = useState(true)
  const [reference, setReference] = useState('')
  const [progress, setProgress] = useState<{ index: number; value: number | null } | null>(null)
  const [errors, setErrors] = useState<string[]>([])
  const plan = system.plans[profiles.indexOf(profile)]
  const chosen = system.models.find((item) => item.id === (model || plan?.model))
  const silence = system.models.find((item) => item.vad)
  const needsModel = !chosen?.installed || (vad && !silence?.installed)
  const runtimeMissing = Object.values(system.diagnostics).some((ok) => !ok)
  const oversize = entries.some((entry) => entry.size > system.upload_limit || entry.size === 0)

  function add(files: File[]) {
    if (progress) return
    const keys = new Set<string>()
    setEntries([...entries, ...files].filter((entry) => {
      const key = [entry.name, entry.size, entry.lastModified].join('/')
      if (keys.has(key)) return false
      keys.add(key)
      return true
    }))
  }

  async function start() {
    if (!entries.length || progress) return
    setErrors([])
    const options: Options = { profile, language, model: model || null, vad, benchmark, reference: benchmark && reference.trim() ? reference.trim() : null }
    const failed: File[] = []
    const problems: string[] = []
    const selected = [...entries]
    for (const [index, file] of selected.entries()) {
      setProgress({ index, value: null })
      try { await upload(file, options, (value) => setProgress({ index, value })) }
      catch (error) { failed.push(file); problems.push(file.name + ' · ' + errorText(error)) }
    }
    setProgress(null)
    setErrors(problems)
    setEntries(failed)
    changed()
    if (!failed.length) window.location.hash = benchmark ? '#benchmarks' : '#queue'
  }

  const languages = [...system.languages].map((key) => [key, languageName(key)] as const).sort((a, b) => a[1].localeCompare(b[1], locale))
  return <>
    <PageTitle title={t(benchmark ? 'benchmarkNew' : 'new')} description={t('newDesc')}/>
    <div className="work-grid">
      <div className="stack">
        <Dropzone compact onFiles={add}/>
        {entries.length > 0 && <section className="panel file-selection">
          <div className="panel-heading"><h2>{t('selected')} <span className="count">{entries.length}</span></h2></div>
          <ul>{entries.map((file, index) => <li key={file.name + file.size + file.lastModified}>
            <span className="media-icon"><FileAudio size={18}/></span>
            <div className="job-name"><strong>{file.name}</strong><small>{bytes(file.size, locale)}</small></div>
            <button disabled={!!progress} className="icon-button" aria-label={t('remove') + ' ' + file.name}
              onClick={() => setEntries(entries.filter((_, n) => n !== index))}><X size={17}/></button>
          </li>)}</ul>
          {progress && <div className="panel-footer">
            <div className="upload-state" role="status"><span>{t('uploadProgress')} · {progress.index + 1}/{entries.length}</span>
              <progress value={progress.value ?? undefined} max={1}/>
            </div></div>}
        </section>}
        {errors.length > 0 && <ul className="notice error" role="alert">{errors.map((error) => <li key={error}>{error}</li>)}</ul>}
        {oversize && <div className="notice error" role="alert">{locale === 'es' ? 'El archivo está vacío o supera el límite' : 'A file is empty or exceeds the limit'} ({bytes(system.upload_limit, locale)}).</div>}
        {benchmark && <section className="panel panel-padding"><label className="field"><span>{t('benchmarkReference')}</span>
          <textarea rows={4} value={reference} maxLength={6000} onChange={(event) => setReference(event.target.value)}/><small>{t('benchmarkReferenceHint')}</small></label></section>}
      </div>
      <div className="stack">
        <section className="panel panel-padding">
          <div className="section-icon"><AudioLines size={19}/></div>
          <div className="form-stack">
            <label className="field"><span>{t('language')}</span><select value={language} onChange={(event) => setLanguage(event.target.value)}>
              <option value="auto">{t('auto')}</option>{languages.map(([key, label]) => <option key={key} value={key}>{label}</option>)}
            </select></label>
            <fieldset className="profiles"><legend>{t('profile')}</legend>
              {profiles.map((item) => <label key={item} className={profile === item ? 'chosen' : ''}>
                <input type="radio" name="profile" value={item} checked={profile === item} onChange={() => setProfile(item)}/>
                <span><strong>{t(item)}</strong><small>{t(item === 'eco' ? 'ecoDesc' : item === 'balanced' ? 'balancedDesc' : 'precisionDesc')}</small></span>
              </label>)}</fieldset>
            <label className="field"><span>{t('model')}</span><select value={model} onChange={(event) => setModel(event.target.value)}>
              <option value="">{t('recommended')}</option>{system.models.filter((item) => !item.vad).map((item) => <option key={item.id} value={item.id}>{item.label} · Q5_1</option>)}
            </select></label>
            <label className="check-row"><input type="checkbox" checked={vad} onChange={(event) => setVad(event.target.checked)}/>
              <span><strong>{t('vad')}</strong><small>{t('vadDesc')}</small></span></label>
            {runtimeMissing ? <a className="notice link-notice" href="#hardware"><Info size={16}/>{t('depsMissing')}<ArrowRight size={15}/></a> :
              needsModel && <a className="notice link-notice" href="#models"><Plus size={16}/>{t('modelMissing')}<ArrowRight size={15}/></a>}
            <button className="button primary w-full justify-center" disabled={!entries.length || needsModel || runtimeMissing || oversize || Boolean(progress) || (!plan && !model)}
              onClick={() => void start()}>{t('transcribe')}<ArrowRight size={17}/></button>
          </div>
        </section>
        {plan && !model && <section className="panel panel-padding"><div className="inline-heading"><Cpu size={18}/><h2>{t('tune')} · {t(profile)}</h2></div>
          <PlanDetails plan={plan} model={chosen}/></section>}
      </div>
    </div>
  </>
}
