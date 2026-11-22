import { createContext, useContext, useEffect, useState, type ReactNode } from 'react'

const en = {
  home: 'Overview', new: 'New transcription', library: 'File library', editor: 'Transcript editor', subtitles: 'Subtitles',
  analysis: 'Local AI', queue: 'Processing queue', history: 'History', benchmarks: 'Benchmarks', hardware: 'Hardware',
  models: 'Models', privacy: 'Privacy', workspace: 'Workspace', system: 'System', local: 'Local workspace',
  private: 'On this device', settings: 'Settings', dark: 'Dark theme', light: 'Light theme', loading: 'Connecting to LocalScribe…',
  welcome: 'Your transcription studio', welcomeDesc: 'Turn audio and video into editable text and subtitles on your computer.',
  uploadTitle: 'Drop audio or video here', uploadDesc: 'Import one recording or add a batch.', browse: 'Choose files',
  recent: 'Recent activity', all: 'View history', emptyRecent: 'Your transcriptions will appear here.', noJobs: 'No matching files.',
  files: 'Files in library', active: 'In queue', completed: 'Completed', running: 'Transcribing', queued: 'Queued',
  cancelled: 'Cancelled', interrupted: 'Interrupted', failed: 'Failed', open: 'Open transcript', cancel: 'Cancel',
  resume: 'Resume', remove: 'Delete', removeConfirm: 'Delete this recording, its transcript and any playback copy from this device?',
  search: 'Search files', more: 'Load older files', import: 'Import', transcribe: 'Start transcription', selected: 'Selected files',
  uploadProgress: 'Copying to local workspace', accepted: 'Queued for transcription', language: 'Audio language',
  auto: 'Automatic', profile: 'AutoTune profile', eco: 'Eco', balanced: 'Balanced', precision: 'Precision',
  ecoDesc: 'Lower CPU and memory use', balancedDesc: 'Balances speed and recognition quality', precisionDesc: 'Prioritizes recognition quality',
  model: 'Whisper model', recommended: 'AutoTune recommendation', vad: 'Voice activity detection',
  vadDesc: 'Use Silero to find speech and skip silences.', modelMissing: 'Install the required models before starting.',
  depsMissing: 'Install the missing tools to start transcribing.', lowMemory: 'The selection needs more free memory or a higher memory budget.',
  supported: 'MP3, WAV, FLAC, M4A, OGG, AAC, MP4, MKV, MOV, WEBM, AVI',
  threads: 'Threads', workers: 'Simultaneous jobs', memory: 'Memory budget', window: 'Audio window', engine: 'Engine',
  tune: 'AutoTune', hardwareDesc: 'LocalScribe measures this computer and uses the results to pick a model, thread count and memory budget.',
  cpu: 'Processor', physical: 'Physical cores', logical: 'Logical CPUs available', available: 'Available RAM',
  gpu: 'Graphics adapters', gpuNone: 'No graphics adapter reported', gpuNote: 'This build transcribes on the CPU.',
  calibration: 'CPU calibration', calibrationDesc: 'Fixed dot product, 90 ms per thread configuration at startup. Rates are millions of arithmetic operations per second.',
  diagnostics: 'Runtime diagnostics', ready: 'Ready', missing: 'Missing', refresh: 'Refresh', refreshPage: 'Reload LocalScribe after adding runtime tools.',
  install: 'Download', installed: 'Installed', absent: 'Available to download', downloadNote: 'Model downloads connect to Hugging Face. They transfer model weights to this computer.',
  uninstallConfirm: 'Remove this model from this computer?', modelDesc: 'Choose a multilingual Whisper model and manage the weights stored on this device.',
  modelFoot: 'Downloads are checked against a SHA-256 checksum. Installed models work offline.', modelSize: 'Download size',
  size: 'Size', file: 'File', date: 'Date added', status: 'Status', duration: 'Duration', actions: 'Actions',
  queueDesc: 'Track processed audio, cancel a job or resume from its last saved window.',
  historyDesc: 'Review completed and interrupted work saved on this computer.',
  libraryDesc: 'Browse the audio and video you have imported into LocalScribe.',
  newDesc: 'Select files, language and processing options.', segmentSearch: 'Search transcript',
  before: 'Previous segment', next: 'Next segment', original: 'Show original', hideOriginal: 'Hide original',
  speaker: 'Speaker', speakerCount: 'Known speaker count (optional)', speakerPlaceholder: 'Add speaker', segment: 'Segment', segments: 'segments',
  saved: 'Saved on this device', saving: 'Saving…', unsaved: 'Changes could not be saved', undo: 'Undo', redo: 'Redo',
  export: 'Export', exported: 'Export saved', time: 'Time', playback: 'Media player', prepare: 'Prepare audio playback',
  preparing: 'Preparing audio…', noSpeech: 'No speech segments were found.', pickTranscript: 'Choose a completed transcript',
  subtitleDesc: 'Review timed cues and export them for a video player or editor.', noTranscript: 'Complete a transcription to use this view.',
  privacyDesc: 'Manage how LocalScribe uses your computer and connects to other software.',
  off: 'Off', on: 'On', offline: 'Works offline', offlineDesc: 'After setup, speech recognition runs with local tools and model weights.',
  loopback: 'Loopback only', loopbackDesc: 'The web interface is served at 127.0.0.1 on your computer.',
  storage: 'Local storage', storageDesc: 'Media, transcript revisions and settings are stored in your LocalScribe data directory.',
  aiOptIn: 'Allow local Ollama', aiOptInDesc: 'Send transcript text to the Ollama server at 127.0.0.1:11434 when you explicitly run an AI action.',
  cloud: 'Cloud providers', cloudDesc: 'This build has no remote AI adapters or telemetry.',
  analysisDesc: 'Work with the text of a completed transcript using an optional local engine.',
  aiDisabled: 'Enable local Ollama in Privacy to use these actions.',
  aiUnavailable: 'Start Ollama on this computer with a local model and cloud features disabled.',
  aiReady: 'Local Ollama', analysisMode: 'Action', general: 'General summary', executive: 'Executive summary',
  key_points: 'Key points', decisions: 'Decisions', tasks: 'Action items', questions: 'Open questions',
  translate: 'Translate', correct: 'Contextual correction', ask: 'Ask about this transcript', speakers: 'Identify speakers',
  optionalSpeakers: 'Automatic speaker identification requires the optional sherpa-onnx setup. Speaker labels can be edited in the transcript.',
  target: 'Output language', askPlaceholder: 'What did the team decide?', run: 'Run locally', analyzing: 'Working locally…',
  evidence: 'Transcript evidence', changes: 'Proposed changes', apply: 'Apply suggestions', result: 'Result',
  benchmarkDesc: 'Measure real transcription runs and compare the results by model.',
  benchmarkNew: 'New benchmark', benchmarkReference: 'Reference transcript for WER (optional)',
  benchmarkReferenceHint: 'Paste the exact words spoken in this sample to calculate word error rate.',
  rtf: 'Real-time factor', rss: 'Sampled peak RAM', cpuUsage: 'CPU use', wer: 'WER', runtime: 'Processing time',
  statsHint: 'RTF is processing time divided by media length. CPU use is shown as a percentage of one core.',
  memoryLabel: 'Memory budget (MiB, 0 for automatic)', memoryDesc: 'The live memory budget applies to jobs that start after you save.',
  save: 'Save settings', close: 'Close', menu: 'Menu', langSwitch: 'Switch language', skip: 'Skip to content',
  none: '—', engines: 'Local engines', newFiles: 'Add files', privacyBadge: 'No cloud required',
} as const

const es: Record<keyof typeof en, string> = {
  home: 'Inicio', new: 'Nueva transcripción', library: 'Biblioteca', editor: 'Editor', subtitles: 'Subtítulos',
  analysis: 'IA local', queue: 'Cola de procesamiento', history: 'Historial', benchmarks: 'Benchmarks', hardware: 'Hardware',
  models: 'Modelos', privacy: 'Privacidad', workspace: 'Espacio de trabajo', system: 'Sistema', local: 'Estudio local',
  private: 'En este equipo', settings: 'Configuración', dark: 'Tema oscuro', light: 'Tema claro', loading: 'Conectando con LocalScribe…',
  welcome: 'Tu estudio de transcripción', welcomeDesc: 'Convierte audio y video en texto editable y subtítulos desde tu equipo.',
  uploadTitle: 'Suelta tu audio o video aquí', uploadDesc: 'Importa una grabación o añade varios archivos.', browse: 'Elegir archivos',
  recent: 'Actividad reciente', all: 'Ver historial', emptyRecent: 'Tus transcripciones aparecerán aquí.', noJobs: 'No hay archivos que coincidan.',
  files: 'Archivos en biblioteca', active: 'En cola', completed: 'Completadas', running: 'Transcribiendo', queued: 'En espera',
  cancelled: 'Cancelada', interrupted: 'Interrumpida', failed: 'Fallida', open: 'Abrir transcripción', cancel: 'Cancelar',
  resume: 'Reanudar', remove: 'Eliminar', removeConfirm: '¿Eliminar de este equipo la grabación, su transcripción y la copia de reproducción?',
  search: 'Buscar archivos', more: 'Cargar archivos anteriores', import: 'Importar', transcribe: 'Iniciar transcripción', selected: 'Archivos seleccionados',
  uploadProgress: 'Copiando al espacio local', accepted: 'Añadido a la cola', language: 'Idioma del audio',
  auto: 'Automático', profile: 'Perfil AutoTune', eco: 'Eco', balanced: 'Equilibrado', precision: 'Precisión',
  ecoDesc: 'Reduce el uso de CPU y memoria', balancedDesc: 'Equilibra velocidad y calidad de reconocimiento', precisionDesc: 'Prioriza la calidad de reconocimiento',
  model: 'Modelo Whisper', recommended: 'Recomendación de AutoTune', vad: 'Detección de voz',
  vadDesc: 'Usa Silero para detectar voz y omitir silencios.', modelMissing: 'Instala los modelos necesarios para empezar.',
  depsMissing: 'Instala las herramientas que faltan para transcribir.', lowMemory: 'La selección requiere más memoria libre o un presupuesto mayor.',
  supported: 'MP3, WAV, FLAC, M4A, OGG, AAC, MP4, MKV, MOV, WEBM, AVI',
  threads: 'Hilos', workers: 'Trabajos simultáneos', memory: 'Presupuesto de memoria', window: 'Ventana de audio', engine: 'Motor',
  tune: 'AutoTune', hardwareDesc: 'LocalScribe mide el equipo y usa los resultados para elegir el modelo, los hilos y el presupuesto de memoria.',
  cpu: 'Procesador', physical: 'Núcleos físicos', logical: 'CPU lógicas disponibles', available: 'RAM disponible',
  gpu: 'Adaptadores gráficos', gpuNone: 'No se detectaron adaptadores gráficos', gpuNote: 'Esta compilación transcribe con la CPU.',
  calibration: 'Calibración de CPU', calibrationDesc: 'Producto escalar fijo, 90 ms por configuración de hilos al iniciar. Las tasas se muestran en millones de operaciones aritméticas por segundo.',
  diagnostics: 'Diagnóstico de dependencias', ready: 'Disponible', missing: 'Falta instalar', refresh: 'Actualizar', refreshPage: 'Reinicia LocalScribe después de instalar las herramientas.',
  install: 'Descargar', installed: 'Instalado', absent: 'Disponible para descargar', downloadNote: 'Las descargas de modelos se conectan a Hugging Face y guardan los pesos en este equipo.',
  uninstallConfirm: '¿Quitar este modelo del equipo?', modelDesc: 'Elige un modelo Whisper multilingüe y administra los pesos guardados en este equipo.',
  modelFoot: 'Se comprueba el SHA-256 de cada descarga. Los modelos instalados funcionan sin conexión.', modelSize: 'Tamaño de descarga',
  size: 'Tamaño', file: 'Archivo', date: 'Fecha de importación', status: 'Estado', duration: 'Duración', actions: 'Acciones',
  queueDesc: 'Consulta el avance real, cancela trabajos o reanúdalos desde la última ventana guardada.',
  historyDesc: 'Revisa las transcripciones terminadas e interrumpidas en este equipo.',
  libraryDesc: 'Explora el audio y video que has importado a LocalScribe.',
  newDesc: 'Selecciona los archivos, el idioma y las opciones de procesamiento.', segmentSearch: 'Buscar en la transcripción',
  before: 'Segmento anterior', next: 'Segmento siguiente', original: 'Ver original', hideOriginal: 'Ocultar original',
  speaker: 'Hablante', speakerCount: 'Cantidad de hablantes conocida (opcional)', speakerPlaceholder: 'Añadir hablante', segment: 'Segmento', segments: 'segmentos',
  saved: 'Guardado en este equipo', saving: 'Guardando…', unsaved: 'No se pudieron guardar los cambios', undo: 'Deshacer', redo: 'Rehacer',
  export: 'Exportar', exported: 'Exportación descargada', time: 'Tiempo', playback: 'Reproductor', prepare: 'Preparar reproducción de audio',
  preparing: 'Preparando audio…', noSpeech: 'No se detectaron segmentos de voz.', pickTranscript: 'Elige una transcripción terminada',
  subtitleDesc: 'Revisa las entradas con tiempo y expórtalas para un reproductor o editor de video.', noTranscript: 'Termina una transcripción para usar esta vista.',
  privacyDesc: 'Controla cómo LocalScribe usa el equipo y se conecta a otras herramientas.',
  off: 'Desactivado', on: 'Activado', offline: 'Funciona sin conexión', offlineDesc: 'Tras la instalación, el reconocimiento utiliza herramientas y pesos almacenados en el equipo.',
  loopback: 'Solo loopback', loopbackDesc: 'La interfaz se sirve en 127.0.0.1 desde tu equipo.',
  storage: 'Almacenamiento local', storageDesc: 'Los archivos, el texto original, las ediciones y los ajustes se guardan en el directorio de datos de LocalScribe.',
  aiOptIn: 'Permitir Ollama local', aiOptInDesc: 'Envía texto al servidor Ollama en 127.0.0.1:11434 cuando ejecutas una acción de IA.',
  cloud: 'Proveedores en la nube', cloudDesc: 'Esta compilación funciona sin adaptadores remotos ni telemetría.',
  analysisDesc: 'Trabaja con el texto de una transcripción terminada mediante un motor local opcional.',
  aiDisabled: 'Activa Ollama local en Privacidad para usar estas acciones.',
  aiUnavailable: 'Inicia Ollama en este equipo con un modelo descargado y las funciones en la nube desactivadas.',
  aiReady: 'Ollama local', analysisMode: 'Acción', general: 'Resumen general', executive: 'Resumen ejecutivo',
  key_points: 'Puntos importantes', decisions: 'Decisiones', tasks: 'Tareas pendientes', questions: 'Preguntas relevantes',
  translate: 'Traducir', correct: 'Corrección contextual', ask: 'Preguntar sobre la transcripción', speakers: 'Identificar hablantes',
  optionalSpeakers: 'La identificación automática requiere la instalación opcional de sherpa-onnx. Puedes editar los hablantes desde el editor.',
  target: 'Idioma del resultado', askPlaceholder: '¿Qué decidió el equipo?', run: 'Ejecutar localmente', analyzing: 'Procesando en el equipo…',
  evidence: 'Fragmentos de respaldo', changes: 'Cambios propuestos', apply: 'Aplicar sugerencias', result: 'Resultado',
  benchmarkDesc: 'Mide transcripciones reales y compara los resultados por modelo.',
  benchmarkNew: 'Nuevo benchmark', benchmarkReference: 'Transcripción de referencia para WER (opcional)',
  benchmarkReferenceHint: 'Pega las palabras exactas de la muestra para calcular la tasa de error.',
  rtf: 'Factor de tiempo real', rss: 'Pico de RAM muestreado', cpuUsage: 'Uso de CPU', wer: 'WER', runtime: 'Tiempo de procesamiento',
  statsHint: 'RTF es el tiempo de procesamiento dividido entre la duración del archivo. El uso de CPU se expresa como porcentaje de un núcleo.',
  memoryLabel: 'Presupuesto de memoria (MiB, 0 para automático)', memoryDesc: 'El presupuesto se aplica a los trabajos que comiencen después de guardar.',
  save: 'Guardar ajustes', close: 'Cerrar', menu: 'Menú', langSwitch: 'Cambiar idioma', skip: 'Ir al contenido',
  none: '—', engines: 'Motores locales', newFiles: 'Añadir archivos', privacyBadge: 'Sin servicios en la nube',
}

export type Key = keyof typeof en
type Locale = 'en' | 'es'
type Theme = 'dark' | 'light'
type Preferences = { locale: Locale; theme: Theme; t: (key: Key) => string; languageName: (code: string) => string; switchLocale: () => void; switchTheme: () => void }
const LocaleContext = createContext<Preferences | null>(null)

export function LocaleProvider({ children }: { children: ReactNode }) {
  const [locale, setLocale] = useState<Locale>(() => {
    const stored = localStorage.getItem('localscribe.locale')
    return stored === 'es' || stored === 'en' ? stored : navigator.language.startsWith('es') ? 'es' : 'en'
  })
  const [theme, setTheme] = useState<Theme>(() => localStorage.getItem('localscribe.theme') === 'light' ? 'light' : 'dark')
  useEffect(() => {
    document.documentElement.lang = locale
    document.documentElement.dataset.theme = theme
    localStorage.setItem('localscribe.locale', locale)
    localStorage.setItem('localscribe.theme', theme)
  }, [locale, theme])
  const languages = new Intl.DisplayNames([locale], { type: 'language' })
  const value: Preferences = {
    locale, theme,
    t: (key) => (locale === 'es' ? es : en)[key],
    languageName: (code) => languages.of(code) ?? code,
    switchLocale: () => setLocale(locale === 'es' ? 'en' : 'es'),
    switchTheme: () => setTheme(theme === 'dark' ? 'light' : 'dark'),
  }
  return <LocaleContext.Provider value={value}>{children}</LocaleContext.Provider>
}

export function useLocale() {
  const preferences = useContext(LocaleContext)
  if (!preferences) throw new Error('Missing locale provider.')
  return preferences
}
