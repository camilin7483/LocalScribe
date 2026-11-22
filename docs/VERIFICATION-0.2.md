# Verificación de LocalScribe 0.2

## Editor y almacenamiento

| Control | Resultado |
|---|---|
| Interacción en Chromium con la red aislada | Pasó con el servidor y Whisper reales. Incluyó importación, audio sincronizado, forma de onda, bucle, búsqueda, edición, guardado automático y paginación. |
| Edición de tiempos | Pasó con coma decimal, tiempo tomado del reproductor, intervalo inválido, foco de teclado, deshacer, rehacer y recuperación de los tiempos originales. |
| Descarga y aviso | Pasó la vuelta al segmento señalado antes de exportar. Los archivos SRT y WebVTT conservaron los milisegundos editados. |
| Compatibilidad del API | Una solicitud antigua que edita texto y hablante sigue funcionando. Las solicitudes con un extremo ausente o un intervalo inválido reciben HTTP 400 y conservan la revisión actual. |
| Base de datos de la 0.1 | Abrí una copia de una base real de la 0.1 sin la tabla nueva. Edité y reinicié el servidor. Persistieron el texto, las nuevas marcas y las originales. SQLite devolvió `ok` en `quick_check` y cero errores de claves foráneas. |
| Pruebas de desarrollo | Pasaron las siete pruebas unitarias Rust, Clippy con advertencias tratadas como errores y la compilación TypeScript y Vite. |

El caso de respuesta en el editor ejecutó 30 búsquedas sobre 3.000 segmentos durante una transcripción de fondo. Las búsquedas correctas pintaron el resultado en 74,8 ms en p95. [Muestras de latencia](measurements/0.2-editor-response.json)

axe-core 4.14.1 señaló una diferencia entre el texto visible y el nombre accesible del selector de idioma, además de campos de hablante inferiores a 24 px. Los dos ajustes pasaron la segunda auditoría en escritorio oscuro, error de tiempo, diálogo de exportación y móvil claro en inglés. El informe automático complementa la prueba de teclado. [Informe axe-core](measurements/0.2-accessibility.json) · [Reglas de axe-core](https://github.com/dequelabs/axe-core/blob/develop/doc/API.md)

## Plataformas

| Distribución local | Control ejecutado |
|---|---|
| Linux x86-64 | Extraje el tarball y ejecuté la aplicación estática en Arch. Pasaron Chromium, los cuatro análisis axe-core, FFmpeg, Whisper y la edición y exportación con la red aislada. |
| Menú Linux | Validé la entrada con Desktop File Utils y lancé por GIO rutas con espacios, porcentaje, comillas, dólar, barra inversa y caracteres Unicode. La entrada también inició la alfa real con la red aislada y entregó la URL local al navegador de prueba. [Desktop Entry Specification](https://specifications.freedesktop.org/desktop-entry/latest/exec-variables.html) |
| Ubuntu Base 24.04.5 | Verifiqué el SHA-256 de la imagen oficial. En un entorno limpio con red aislada arrancaron el servidor y Whisper y se sirvió el frontend. [Ubuntu Base](https://cdimage.ubuntu.com/ubuntu-base/releases/24.04/release/) |
| Windows x86-64 | Verifiqué y extraje el ZIP. En Wine 11.22 la aplicación anunció 0.2.0-alpha.1, seleccionó CPU (AVX2) y pasó la transcripción en español con FFmpeg, la validación de tiempos y cinco exportaciones. El flujo de Chromium contra el servidor Windows también pasó en Wine, con forma de onda, edición de tiempos, aviso de exportación y paginación. |
| Motor de Windows | La compilación previa etiquetada AVX2 ejecutaba instrucciones genéricas. El motor recompilado informa AVX2, FMA y BMI2 en tiempo de ejecución. En dos muestras y cuatro hilos produjo exactamente el mismo texto y los mismos tiempos que el binario previo. [Comparación en Wine](measurements/0.2-windows-avx2.json) |

Los archivos locales de la alfa contienen motores CPU. El protocolo persistente y Vulkan están disponibles en compilaciones de laboratorio desde el código fuente. Quedan pendientes los instaladores nativos de Windows 10 y 11, el escritorio de Ubuntu en una instalación limpia, una ejecución del workflow publicado y la firma de distribución.


## Demo

El MP4 se grabó en la alfa real. Muestra importación, cola, hardware, forma de onda, ajuste de tiempos, descarga SRT y las funciones de resumen, traducción y hablantes respaldadas por Ollama y sherpa-onnx locales. FFprobe contó 900 fotogramas H.264 a 30 FPS, 1920 × 1080 y una duración de 30,000 s. La grabación registró únicamente peticiones del navegador a loopback.
