import { ArrowRight, Check, CircuitBoard, Cpu, Database, Download, HardDrive, LockKeyhole, Radio, ShieldCheck, Trash2 } from 'lucide-react'
import { useState } from 'react'
import { bytes, errorText, request } from './api'
import { useLocale } from './i18n'
import type { Model, Profile, System } from './types'
import { ModelStatus, PageTitle, PlanDetails, profiles } from './ui'

export function HardwareView({ system, changed }: { system: System; changed: () => void }) {
  const { t, locale } = useLocale()
  const [profile, setProfile] = useState<Profile>('balanced')
  const [limits, setLimits] = useState(system.settings)
  const [notice, setNotice] = useState('')
  const [failed, setFailed] = useState(false)
  const { hardware } = system
  const plan = system.plans[profiles.indexOf(profile)]

  async function save() {
    setNotice('')
    try {
      await request('/settings', { method: 'PATCH', body: JSON.stringify({
        memory_mib: limits.memory_mib, max_workers: limits.max_workers,
      }) })
      setFailed(false); setNotice(t('saved')); changed()
    } catch (error) { setFailed(true); setNotice(errorText(error)) }
  }

  return <>
    <PageTitle title={t('hardware')} description={t('hardwareDesc')} action={<button className="button secondary" onClick={changed}>{t('refresh')}</button>}/>
    <div className="work-grid">
      <div className="stack">
        <section className="panel panel-padding hardware-panel"><div className="inline-heading"><Cpu size={20}/><h2>{t('cpu')}</h2></div>
          <h3>{hardware.cpu}</h3>
          <dl className="hardware-stats">
            <div><dt>{t('physical')}</dt><dd>{hardware.physical ?? '—'}</dd></div>
            <div><dt>{t('logical')}</dt><dd>{hardware.logical}</dd></div>
            <div><dt>{t('available')}</dt><dd>{bytes(hardware.available_mib * 1048576, locale)} <small>/ {bytes(hardware.ram_mib * 1048576, locale)}</small></dd></div>
          </dl>
          <div className="hardware-gpu"><CircuitBoard size={23}/><div><h4>{t('gpu')}</h4><p>{hardware.adapters.join(' · ') || t('gpuNone')}</p>
            <small>{t('gpuNote')} {hardware.arch}{hardware.avx2 ? ' · AVX2' : ''}</small></div></div>
        </section>
        <section className="panel panel-padding"><div className="inline-heading"><Radio size={18}/><h2>{t('calibration')}</h2></div>
          <p className="hint">{t('calibrationDesc')}</p>
          <div className="calibration">{hardware.calibration.map((result) => <div key={result.threads}>
            <span>{result.threads} {t('threads').toLowerCase()}</span><meter min={0} max={Math.max(...hardware.calibration.map((row) => row.million_ops_per_second))} value={result.million_ops_per_second}/>
            <strong className="mono">{Math.round(result.million_ops_per_second).toLocaleString(locale)}</strong></div>)}</div>
        </section>
      </div>
      <div className="stack">
        <section className="panel panel-padding"><div className="inline-heading"><CircuitBoard size={18}/><h2>{t('tune')}</h2></div>
          <div className="toggle-group" aria-label={t('profile')}>{profiles.map((item) => <button aria-pressed={item === profile} className={item === profile ? 'chosen' : ''} key={item}
            onClick={() => setProfile(item)}>{t(item)}</button>)}</div>
          {plan ? <PlanDetails plan={plan} model={system.models.find((item) => item.id === plan.model)}/> : <p className="hint">{t('lowMemory')}</p>}
        </section>
        <section className="panel panel-padding"><div className="inline-heading"><HardDrive size={18}/><h2>{t('diagnostics')}</h2></div>
          <dl className="key-values">{Object.entries(system.diagnostics).map(([tool, ok]) => <div key={tool}>
            <dt>{tool === 'whisper' ? 'whisper.cpp' : tool}</dt><dd className={ok ? 'good' : 'warn'}>{t(ok ? 'ready' : 'missing')}</dd></div>)}</dl>
          {Object.values(system.diagnostics).some((value) => !value) && <p className="hint mt-3">{t('refreshPage')}</p>}
        </section>
        <form className="panel panel-padding form-stack" onSubmit={(event) => { event.preventDefault(); void save() }}>
          <label className="field"><span>{t('memoryLabel')}</span><input type="number" min={0} max={24576} value={limits.memory_mib} onChange={(event) => setLimits({ ...limits, memory_mib: Number(event.target.value) })}/></label>
          <label className="field"><span>{t('workers')}</span><select value={limits.max_workers} onChange={(event) => setLimits({ ...limits, max_workers: Number(event.target.value) })}><option>1</option><option>2</option></select></label>
          <p className="hint">{t('memoryDesc')}</p><button className="button secondary"><Check size={16}/>{t('save')}</button>
          {notice && <p role={failed ? 'alert' : 'status'} className={'notice ' + (failed ? 'error' : '')}>{notice}</p>}
        </form>
      </div>
    </div>
  </>
}

export function ModelsView({ system, changed }: { system: System; changed: () => void }) {
  const { t, locale } = useLocale()
  const [error, setError] = useState('')
  const [pending, setPending] = useState<string | null>(null)

  async function act(model: Model) {
    if (model.installed && !window.confirm(t('uninstallConfirm'))) return
    setPending(model.id); setError('')
    try {
      await request('/models/' + model.id, model.installed ? { method: 'DELETE' } : { method: 'POST', headers: { 'X-LocalScribe-Download': '1' } })
      changed()
    } catch (err) { setError(errorText(err)) }
    finally { setPending(null) }
  }

  return <>
    <PageTitle title={t('models')} description={t('modelDesc')}/>
    <p className="notice mb-5"><ShieldCheck size={18}/>{t('downloadNote')}</p>
    {error && <p className="notice error mb-5" role="alert">{error}</p>}
    <div className="model-grid">{system.models.map((model) => <section key={model.id} className="panel panel-padding model-card">
      <div className="section-icon"><Database size={20}/></div>
      <div><h2>{model.label}</h2><p className="hint">{model.vad ? t('vad') : 'Q5_1 · ' + bytes(model.size, locale)}</p></div>
      <ModelStatus model={model}/>
      <button disabled={model.downloading || pending !== null} className={'button ' + (model.installed ? 'secondary' : 'primary')} onClick={() => void act(model)}>
        {model.installed ? <Trash2 size={15}/> : <Download size={15}/>} {t(model.installed ? 'remove' : 'install')}</button>
      {model.error && <p className="inline-error" role="alert">{model.error}</p>}
    </section>)}</div>
    <p className="hint mt-5">{t('modelFoot')}</p>
  </>
}

export function PrivacyView({ system, changed }: { system: System; changed: () => void }) {
  const { t } = useLocale()
  const [updating, setUpdating] = useState(false)
  const [error, setError] = useState('')
  async function toggle() {
    setUpdating(true); setError('')
    try {
      const settings = { ollama_enabled: !system.settings.ollama_enabled }
      await request('/settings', { method: 'PATCH', body: JSON.stringify(settings) })
      changed()
    } catch (err) { setError(errorText(err)) }
    finally { setUpdating(false) }
  }
  return <>
    <PageTitle title={t('privacy')} description={t('privacyDesc')}/>
    <div className="privacy-grid">
      {([
        [LockKeyhole, 'loopback', 'loopbackDesc'],
        [ShieldCheck, 'offline', 'offlineDesc'],
        [Database, 'storage', 'storageDesc'],
        [Radio, 'cloud', 'cloudDesc'],
      ] as const).map(([Icon, title, description]) => <section className="panel panel-padding" key={title}>
        <div className="section-icon"><Icon size={20}/></div><h2>{t(title)}</h2><p className="hint">{t(description)}</p></section>)}
    </div>
    <section className="panel panel-padding privacy-optin mt-5">
      <div><h2>{t('aiOptIn')}</h2><p className="hint">{t('aiOptInDesc')}</p></div>
      <button className="switch" role="switch" aria-checked={system.settings.ollama_enabled} aria-label={t('aiOptIn')}
        disabled={updating} onClick={() => void toggle()}><span/></button>
      {error && <p className="notice error" role="alert">{error}</p>}
    </section>
    <a href="#history" className="button secondary mt-5">{t('history')}<ArrowRight size={16}/></a>
  </>
}
