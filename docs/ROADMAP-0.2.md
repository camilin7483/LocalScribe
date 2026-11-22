# LocalScribe 0.2 — plan de producto e ingeniería

El objetivo es que una persona pueda pasar de una grabación privada a un texto y unos subtítulos revisados con menos espera, menos correcciones y sin salir del editor. El primer público son periodistas, creadores y equipos bilingües con portátiles Windows o Linux. «Ser el mejor» se evaluará en ese recorrido con el mismo corpus, equipo y protocolo para todos los productos comparados.

## Punto de partida

En el i3‑1215U, la 0.1 obtuvo un RTF mediano de 0,300 con Tiny, 0,902 con Base y 1,432 con Small. Tiny en automático dio 100% de WER frente a 7,7% de Base y Small en la misma muestra sintética de 11,786 s. Esto identifica una regresión que debe capturarse, no una comparación general de modelos. El código vuelve a lanzar FFmpeg e inicializa Whisper por cada ventana, dibuja intervalos de texto en la pista y monta todos los campos de edición simultáneamente. El historial por ventanas y la compatibilidad CPU ya tienen cobertura de trabajo real. Windows nativo sigue pendiente. [Mediciones 0.1](benchmark-i3.json) · [Verificación 0.1](VERIFICATION.md)

## Referencias y decisiones

| Referencia | Capacidad documentada | Decisión para LocalScribe |
|---|---|---|
| Vibe | Transcripción local con un proceso de inferencia separado y soporte Vulkan en Linux y Windows. | Medir inferencia persistente aislada y Vulkan junto a CPU. Conservar resultados parciales, cancelación y recuperación cuando el proceso falle. [Arquitectura de Vibe](https://github.com/thewh1teagle/vibe/blob/main/server/README.md) |
| Subtitle Edit | Navegación y edición sobre forma de onda y diagnósticos de velocidad de lectura. | Integrar forma de onda real, saltos a incidencias y revisión de tiempos en el editor principal. [Historial de cambios de Subtitle Edit](https://github.com/SubtitleEdit/subtitleedit/blob/main/change-log.txt) |
| faster-whisper | CTranslate2, inferencia por lotes y comparaciones publicadas de rendimiento. | Usar como comparador opcional con parámetros equivalentes. Medir coste de decodificación y carga antes de añadir otro motor al producto. [Metodología de faster-whisper](https://github.com/SYSTRAN/faster-whisper) |
| whisper.cpp | Contexto reutilizable, detección de idioma y backend Vulkan documentados. | Mantener el motor actual y prototipar un proceso propietario del contexto C++. Seleccionar por pruebas end-to-end con audio real y detectar el backend que terminó cada ventana. [API C](https://github.com/ggml-org/whisper.cpp/blob/v1.9.5/include/whisper.h) · [Backends de whisper.cpp](https://github.com/ggml-org/whisper.cpp#vulkan-gpu-support) |

Las comparaciones anteriores son de capacidades documentadas. El ranking requiere mediciones propias y publicación de versiones, ajustes, corpus y fallos.

## Experiencia de uso

El espacio de trabajo prioriza una banda de transporte con señal real, texto editable al centro y revisión contextual a la derecha. Biblioteca y cola conservan sus URLs. El panel de revisión muestra el segmento en reproducción, incidencias que se pueden localizar y controles para validar y exportar. En pantallas estrechas el panel baja bajo el texto. La navegación teclado se mantiene en el orden visual.

```text
Archivo y estado de guardado                         Exportar
Forma de onda / posición / velocidad / segmento en bucle
Transcripción editable y búsqueda     Revisión del subtítulo
```

Paleta de referencia para el editor oscuro: tinta #E9EFF4, fondo #10151D, papel #182230, línea #334358, señal #79E2C5 y aviso #F3CD8B. Mantener Manrope para texto y Plex Mono para tiempos. El color codifica reproducción y revisión con texto e indicadores accesibles. En el tema claro se ajustan todos los pares de contraste. Usar foco visible y pruebas manuales de zoom, teclado, lector de pantalla y reducción de movimiento conforme a WCAG 2.2 AA. [WCAG 2.2](https://www.w3.org/TR/WCAG22/)

## Orden de implementación y puertas de salida

| Orden | Entrega | Evidencia para cerrar |
|---|---|---|
| 1 | Captura de la línea base y regresiones de reconocimiento. Añadir referencias humanas con permisos, español e inglés con acentos, code-switching, silencio, ruido, entrevistas y pistas de 30–120 min. Guardar ajustes, commits, versiones, fallos y mediciones por archivo. | Repetir corridas frías y calientes de CLI y producto con los mismos modelos. Reportar WER/CER por idioma y escenario, latencia al primer texto, RTF p50/p95, RSS propio y del árbol, CPU y errores. La referencia de 0.1 permanece como comparación histórica. |
| 2 | Estudio integrado. Forma de onda acotada y derivada del audio real, transporte con velocidades y repetición, seguimiento opcional, revisión de lectura y solapamientos, abrir incidencias, subtítulo activo y verificación antes de exportar. Paginar o virtualizar con foco estable al editar. | E2E con audio real, búsqueda, edición, reproducción, autosave y exportación. Mantener los controles interactivos durante el análisis. Tres pruebas de tarea por idioma con usuarios o protocolo moderado equivalente; medir tiempo a primer subtítulo corregido y errores. |
| 3 | Calidad de ASR. Revisar VAD, detección temprana con señal suficiente, tramos sin habla, duplicados en límites y fallback conservador. Respetar idioma/modelo manual; mostrar cuándo se reintentó y por qué. | Tiny en automático recupera la muestra de regresión con WER <35% o deriva explícitamente a un modelo compatible dentro del presupuesto. Base y Small no empeoran >1 punto absoluto en el corpus por idioma. Cero texto en silencio puro del corpus; medir alucinación en música y pérdida de habla suave aparte. |
| 4 | Optimización. Comparar CLI con contexto persistente, pipeline PCM incremental y GPU Vulkan. Medir perfiles y consumo por trabajo; persistir ajustes por versión, modelo, dispositivo y controlador. Validar fallos o bloqueo de GPU con vuelta a CPU desde el último checkpoint. | En i3 aspirar a ≥25% menos tiempo mediano para 30 min con igual modelo y calidad, ≤10% más RSS máximo del árbol y p95 de respuesta de UI <100 ms. Permitir Vulkan automático únicamente donde sea ≥15% más rápido con calidad equivalente y pruebas de recuperación. Publicar resultados negativos. |
| 5 | Producción. Validar integridad de datos, migración con respaldo y recuperación, ejecución sin red, empaquetado verificado y descargas comprobadas. Ejecución de CI y pruebas nativas en Win10/11, Ubuntu y Arch. | Recuperación repetida al cortar un trabajo y al actualizarlo. Matriz con nivel de prueba y logs por plataforma. Ejecución de CI a partir de un repositorio autorizado. Firma y distribución Windows dependen de un certificado y canal autorizados. |

Los números de las puertas son objetivos propuestos para la 0.2. Fijar corpus y metodología antes de comparar. Si una variante no alcanza su puerta, registrar el resultado y mantener la variante anterior como fallback. Evitar nuevas promesas de compatibilidad o de liderazgo hasta reunir esa evidencia.

## Primer incremento verificado

- El editor deriva 768 muestras de pico de audio local con FFmpeg. La lectura PCM es incremental, la respuesta está acotada y existe un único análisis de señal simultáneo. El transporte ofrece velocidad y repetición. La revisión señala tiempos inválidos, solapamientos y más de 20 caracteres por segundo. La lista monta hasta 100 segmentos por página y conserva búsqueda global, edición y guardado.
- La referencia española produjo cero segmentos con Tiny en automático, Silero activo y una predicción italiana con probabilidad 0,292. Sin Silero, Tiny detectó español y mantuvo varios errores de reconocimiento. La selección Eco ahora prefiere Base cuando cabe en el presupuesto, conserva Tiny como fallback de memoria y conserva también la selección explícita. El E2E del API obtuvo 11,5% de WER con Eco automático y Base en esa muestra. Se necesita un corpus humano antes de ajustar VAD o los umbrales.
- La prueba de navegador pasó con red aislada. Comprobó que el canvas recibe señal, la velocidad cambia en el reproductor, el bucle se activa y desactiva y la búsqueda encuentra texto fuera de la página visible. La integración verificó importación real, edición, exportación y varios contenedores.
- Comparé un proceso por archivo con un solo proceso para los mismos WAV español e inglés, Base Q5_1, Silero y cuatro hilos AVX2. La mediana de dos recorridos bajó de 9,980 s a 8,896 s, aproximadamente 11%. Los idiomas, textos y tiempos coincidieron en los dos pares. Esta medición excluye FFmpeg y la cola. Justifica un prototipo de contexto reutilizable, todavía sin demostrar la meta del 25% en grabaciones largas. [Datos de la comparación](measurements/0.2-whisper-context.json)

```sh
python scripts/compare_whisper_context.py --whisper PATH_WHISPER --model PATH_BASE --vad PATH_SILERO --repeats 2 tests/fixtures/es.wav tests/fixtures/en.wav
```

Siguiente entrega de ingeniería: instrumentar tiempos de preparación, búsqueda y decodificación de FFmpeg, inferencia y persistencia por ventana. Probar un worker con contexto reutilizable aislado y someterlo a reinicios, cancelación y ventanas sin habla. Comparar una sesión de 30 minutos antes de activar un backend por defecto.

## Inferencias y segundo incremento

La compilación nativa añade un protocolo de un byte en tuberías privadas al CLI verificado de whisper.cpp. El contexto vive durante el trabajo de varias ventanas. El orquestador mantiene los checkpoints por ventana y reintenta con CLI aislado si falla el proceso persistente. El modo se activa con `LOCALSCRIBE_WHISPER_PERSISTENT=1`; la ruta predeterminada sigue siendo el CLI aislado.

En la comparación end-to-end de 72,946 s, ambos modos devolvieron el mismo texto y tiempos. El aislado tardó 24,980 s y el persistente 25,535 s. El pico estimado de los procesos hijos subió de 158,1 a 195,9 MiB. Había compilación y otras pruebas en el mismo equipo. Esta corrida de diagnóstico no justifica habilitar el modo como predeterminado. Un binario sin el protocolo volvió al CLI automáticamente y produjo la misma salida. La prueba de 93 s canceló el trabajo, reinició el servidor y reanudó con las marcas ya guardadas intactas. [Medición de la sesión corta](measurements/0.2-worker-short.json)

El editor añade seguimiento opcional con protección del campo que tiene foco, subtítulo activo en el panel y superposición sobre video. Los tiempos de inspección, decodificación e inferencia se guardan como métricas opcionales compatibles con trabajos anteriores. Las nuevas métricas distinguen ventanas persistentes, ventanas aisladas, silencios digitales exactos y reintentos. El pico combinado es una estimación construida con la muestra del worker inactivo y el pico del decodificador. El proceso principal y el navegador quedan fuera de esa cifra.

### Corpus y comparación de dispositivos

La muestra humana usa 12 grabaciones distintas de cada partición de prueba `es_419` y `en_us` de FLEURS. El script fija la revisión y registra el SHA‑256 de cada archivo. FLEURS contiene lectura de frases. La sesión larga construida a partir de varias lecturas mide rendimiento y equivalencia, con pausas introducidas y secuencias repetidas. El corpus necesita entrevistas, voces mezcladas, ruido y grabaciones humanas largas independientes para cerrar todas las puertas. [Google Research](https://research.google/pubs/fleurs-few-shot-learning-evaluation-of-universal-representations-of-speech/) · [FLEURS, CC BY 4.0](https://huggingface.co/datasets/google/fleurs/tree/70bb2e84b976b7e960aa89f1c648e09c59f894dd)

```sh
python scripts/fetch_fleurs.py CORPUS
python scripts/evaluate_corpus.py --binary APP --models MODELS CORPUS/manifest.json
python scripts/prepare_corpus_session.py CORPUS/manifest.json SESSION.flac --seconds 1800
python tests/worker.py --binary APP --models MODELS --media SESSION.flac --timeout 2400
python scripts/compare_whisper_devices.py --whisper CLI_VULKAN --model MODEL --vad SILERO CORPUS/manifest.json
```

El equipo de referencia informa Intel Graphics ADL GT2 con Mesa 26.2.7. El comparador exige que Whisper informe uso efectivo del backend Vulkan en cada prueba GPU. La selección automática queda sujeta a medidas de velocidad y calidad repetibles, memoria, recuperación y pruebas por controlador.

En Chromium con red aislada, 30 búsquedas sobre 3.000 segmentos sintéticos durante una transcripción real de fondo dieron 37,2 ms en p50, 74,8 ms en p95 y 107,5 ms como máximo. La duración va del evento de entrada al segundo frame de animación. Cada muestra esperaba el resultado correcto. Este caso cubre búsqueda en un editor abierto; la apertura inicial, otros equipos y tareas de usuarios requieren medición propia. [Muestras de latencia](measurements/0.2-editor-response.json)

La prueba posterior con silencio digital confirmó dos ventanas inferidas y una omitida en cada modo. CLI, worker persistente y reintento con binario antiguo generaron exactamente los mismos cuatro segmentos con sus marcas de tiempo. [Datos del control de silencio](measurements/0.2-worker-silence.json)
