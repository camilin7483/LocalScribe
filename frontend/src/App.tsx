import {
  Activity, ArrowDownToLine, ArrowRight, AudioLines, Captions, ChartNoAxesCombined, Clock3, Cpu, Database, FileAudio,
  FolderOpen, Globe2, LayoutDashboard, ListFilter, Menu, MoonStar, PanelLeftClose, Plus, ShieldCheck, Sparkles, Sun, TextCursorInput,
} from 'lucide-react'
import { useCallback, useEffect, useState } from 'react'
import { AnalysisView } from './analysis'
import { bytes, errorText, request } from './api'
import { EditorView, SubtitlesView } from './editor'
import { useLocale } from './i18n'
import { ImportView } from './import'
import { JobsView, RecentRows, useJobs } from './jobs'
import { HardwareView, ModelsView, PrivacyView } from './system'
import type { System, View } from './types'
import { BrandMark, Dropzone, PageTitle } from './ui'

const primary = [
  ['home', LayoutDashboard], ['new', Plus], ['library', FolderOpen], ['editor', TextCursorInput],
  ['subtitles', Captions], ['analysis', Sparkles], ['queue', ListFilter], ['history', Clock3],
] as const
const secondary = [['benchmarks', ChartNoAxesCombined], ['hardware', Cpu], ['models', Database], ['privacy', ShieldCheck]] as const

function location() {
  const [value, argument] = window.location.hash.slice(1).split('/')
  const view = [...primary, ...secondary].find(([entry]) => entry === value)?.[0] ?? 'home'
  return { view, argument }
}

export function App() {
  const { t, locale, theme, switchLocale, switchTheme } = useLocale()
  const [route, setRoute] = useState(location)
  const [system, setSystem] = useState<System | null>(null)
  const [error, setError] = useState('')
  const [entries, setEntries] = useState<File[]>([])
  const [menu, setMenu] = useState(false)
  const refresh = useCallback(async () => {
    try { setSystem(await request<System>('/system')); setError('') }
    catch (err) { setError(errorText(err)) }
  }, [])

  useEffect(() => {
    const changed = () => { setRoute(location()); setMenu(false); window.scrollTo(0, 0) }
    window.addEventListener('hashchange', changed)
    return () => window.removeEventListener('hashchange', changed)
  }, [])
  useEffect(() => { void refresh() }, [route.view, refresh])
  const working = !!system?.counts.active || !!system?.models.some((model) => model.downloading)
  useEffect(() => {
    if (!working) return
    const interval = window.setInterval(() => void refresh(), 850)
    return () => window.clearInterval(interval)
  }, [working, refresh])
  useEffect(() => { document.title = (route.view === 'home' ? '' : t(route.view) + ' · ') + 'LocalScribe' }, [route.view, t])

  const changed = () => { void refresh() }
  function importFiles(files: File[]) {
    if (!files.length) return
    setEntries(files)
    window.location.hash = '#new'
  }
  function content(view: View, facts: System) {
    switch (view) {
      case 'home': return <Dashboard system={facts} onFiles={importFiles}/>
      case 'new': return <ImportView key={route.argument ?? 'normal'} system={facts} entries={entries} setEntries={setEntries} benchmark={route.argument === 'benchmark'} changed={changed}/>
      case 'library': case 'queue': case 'history': case 'benchmarks': return <JobsView key={view} view={view} changed={changed}/>
      case 'editor': return <EditorView id={route.argument}/>
      case 'subtitles': return <SubtitlesView/>
      case 'analysis': return <AnalysisView system={facts}/>
      case 'hardware': return <HardwareView system={facts} changed={changed}/>
      case 'models': return <ModelsView system={facts} changed={changed}/>
      case 'privacy': return <PrivacyView system={facts} changed={changed}/>
    }
  }

  return <div className="shell">
    <a className="skip-link" href="#main-content" onClick={(event) => { event.preventDefault(); document.getElementById('main-content')?.focus() }}>{t('skip')}</a>
    <aside className={'sidebar' + (menu ? ' visible' : '')}>
      <a href="#home" className="brand"><BrandMark/><span><strong>LocalScribe</strong><small>{t('local')}</small></span></a>
      <nav aria-label={t('workspace')}>
        <div className="nav-group"><h2>{t('workspace')}</h2>{primary.map(([view, Icon]) =>
          <a key={view} href={'#' + view} aria-current={route.view === view ? 'page' : undefined}><Icon size={17}/><span>{t(view)}</span>
            {view === 'queue' && !!system?.counts.active && <small className="nav-count">{system.counts.active}</small>}</a>)}</div>
        <div className="nav-group"><h2>{t('system')}</h2>{secondary.map(([view, Icon]) =>
          <a key={view} href={'#' + view} aria-current={route.view === view ? 'page' : undefined}><Icon size={17}/><span>{t(view)}</span></a>)}</div>
      </nav>
      <a className="sidebar-foot" href="#privacy"><ShieldCheck size={19}/><span><strong>{t('private')}</strong><small>LocalScribe {system?.version ?? ''}</small></span><ArrowRight size={14}/></a>
    </aside>
    {menu && <button className="menu-scrim" aria-label={t('close')} onClick={() => setMenu(false)}/>}
    <div className="main-shell">
      <header className="topbar">
        <button className="icon-button menu-toggle" aria-expanded={menu} aria-label={t('menu')} onClick={() => setMenu(!menu)}>{menu ? <PanelLeftClose size={21}/> : <Menu size={21}/>}</button>
        <div className="topbar-path"><span>{t('workspace')}</span><span className="path-divider">/</span><strong>{t(route.view)}</strong></div>
        <div className="topbar-tools">
          <a className="local-status" href="#privacy"><span/>{t('private')}</a>
          <button className="icon-button locale-button" aria-label={t('langSwitch')} title={t('langSwitch')} onClick={switchLocale}><Globe2 size={17}/><span>{locale.toUpperCase()}</span></button>
          <button className="icon-button" aria-label={t(theme === 'dark' ? 'light' : 'dark')} title={t(theme === 'dark' ? 'light' : 'dark')} onClick={switchTheme}>{theme === 'dark' ? <Sun size={18}/> : <MoonStar size={18}/>}</button>
        </div>
      </header>
      <main id="main-content" className="content" tabIndex={-1}>
        {error && <div className="notice error mb-4" role="alert">{error}<button className="button small secondary" onClick={changed}>{t('refresh')}</button></div>}
        {system ? content(route.view, system) : !error && <p className="notice" role="status">{t('loading')}</p>}
      </main>
    </div>
  </div>
}

function Dashboard({ system, onFiles }: { system: System; onFiles: (files: File[]) => void }) {
  const { t, locale } = useLocale()
  const { jobs, loading } = useJobs()
  const tuned = system.plans[1]
  const currentModel = system.models.find((model) => model.id === tuned?.model)
  const counts = [
    ['files', system.counts.files, FileAudio, '#library'], ['active', system.counts.active, Activity, '#queue'],
    ['completed', system.counts.completed, Captions, '#history'],
  ] as const
  return <>
    <PageTitle title={t('welcome')} description={t('welcomeDesc')} action={<a className="button primary" href="#new"><Plus size={19}/>{t('new')}</a>}/>
    <div className="overview-grid">
      <div className="stack">
        <Dropzone onFiles={onFiles}/>
        <section className="panel"><div className="panel-heading"><div className="inline-heading"><Clock3 size={18}/><h2>{t('recent')}</h2></div>
          <a href="#history" className="text-link">{t('all')}<ArrowRight size={15}/></a></div>
          <RecentRows jobs={jobs} loading={loading}/>
        </section>
      </div>
      <div className="stack">
        <section className="panel hardware-summary">
          <div className="inline-heading"><div className="section-icon"><Cpu size={21}/></div><div><h2>{t('tune')}</h2><p>{t('recommended')}</p></div></div>
          <strong className="device-name">{system.hardware.cpu}</strong>
          {tuned && <div className="summary-memory">
            <div><span>{t(tuned.profile)}</span><strong>{tuned.threads} {t('threads').toLowerCase()}</strong></div>
            <meter aria-label={t('available')} min={0} max={system.hardware.ram_mib || 1} value={system.hardware.available_mib}/>
            <div><small>{t('available')}</small><strong>{bytes(system.hardware.available_mib * 1048576, locale)}</strong></div>
          </div>}
          <a href="#hardware" className="text-link">{t('hardware')}<ArrowRight size={15}/></a>
        </section>
        <section className="panel model-summary">
          <div className="inline-heading"><div className="section-icon violet"><AudioLines size={21}/></div><div><h2>{currentModel?.label ?? t('model')}</h2><p>{t('model')} {tuned?.quantization}</p></div></div>
          {currentModel && <span className={'pill ' + (currentModel.installed ? 'status-completed' : '')}>
            <span aria-hidden="true"/>{t(currentModel.installed ? 'installed' : 'missing')}</span>}
          <a href="#models" className="text-link">{currentModel?.installed ? <Database size={15}/> : <ArrowDownToLine size={15}/>} {t('models')}<ArrowRight size={15}/></a>
        </section>
      </div>
    </div>
    <div className="stats-strip">{counts.map(([label, value, Icon, href]) => <a href={href} key={label}>
      <span className="stat-icon"><Icon size={19}/></span><span><small>{t(label)}</small><strong>{value.toLocaleString(locale)}</strong></span><ArrowRight size={16}/></a>)}</div>
  </>
}
