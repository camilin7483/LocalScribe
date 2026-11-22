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

La alfa local usa los motores CPU portables verificados en la 0.1. El protocolo persistente y Vulkan se prueban con compilaciones explícitas del código fuente. Los instaladores nativos de Windows 10 y 11, el escritorio de Ubuntu en una instalación limpia, una ejecución del workflow publicado y la firma de distribución requieren sus entornos de prueba.
