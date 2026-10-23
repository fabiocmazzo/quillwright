# Textos de la interfaz de Quillwright en español.
# El original es ../en/quillwright.ftl, que explica las variables. Conserve los mismos ids.

## Counts

words = { $count ->
    [one] { $words } palabra
   *[other] { $words } palabras
}
thousands = { $count } mil

## Menus

menu-file = Archivo
menu-view = Ver
menu-assistant = Asistente
menu-tools = Herramientas
menu-help = Ayuda
menu-lint = Revisar Markdown
menu-format = Formatear Markdown
menu-shortcuts = Atajos de Teclado
menu-add-research = Añadir Archivos de Investigación...
    .hint = Copia PDF, imágenes y notas a la carpeta de investigación del proyecto
menu-new-project = Nuevo Proyecto...
menu-open-project = Abrir Proyecto...
menu-project-home = Inicio del Proyecto
menu-new-blank = Nuevo Documento en Blanco
menu-open-file = Abrir Archivo...
menu-save = Guardar
menu-save-as = Guardar Como...
menu-autosave = Guardado automático
    .hint = Guarda el archivo abierto poco después de que deja de escribir
menu-history = Historial de Versiones...
    .hint = Versiones anteriores de este archivo, guardadas en este equipo
menu-export = Exportar Manuscrito...
menu-exit = Salir
menu-assistant-panel = Panel del Asistente
menu-assistant-settings = Ajustes del Asistente...
menu-theme = Tema
menu-theme-system = Como el sistema
menu-focus = Modo Concentración
menu-sidebar = Barra Lateral
menu-search = Buscar en el Proyecto...
menu-reference = Panel de Referencia
    .hint = El archivo de investigación que abrió por última vez
menu-typewriter = Desplazamiento de Máquina de Escribir
    .hint = Mantiene la línea que escribe en el centro de la pantalla
menu-dim = Atenuar Otros Párrafos
    .hint = Atenúa todo excepto el párrafo que está escribiendo
menu-line-numbers = Números de Línea
menu-larger = Texto Más Grande
menu-smaller = Texto Más Pequeño
menu-default-size = Tamaño de Texto Predeterminado
menu-session-goal = Meta de la sesión (palabras)
    .hint = 0 oculta la meta
menu-language = Idioma de la Interfaz
menu-language-system = Como el sistema

## Keyboard shortcuts (Help → Keyboard Shortcuts)

shortcuts-title = Atajos de Teclado
shortcut-focus = Modo concentración (Esc para salir)
shortcut-sidebar = Mostrar / ocultar la barra lateral
shortcut-assistant = Panel del asistente de IA
shortcut-export = Exportar el manuscrito
shortcut-send = Enviar la petición al asistente
shortcut-typewriter = Desplazamiento de máquina de escribir
shortcut-dim = Atenuar otros párrafos
shortcut-text-size = Texto más grande / más pequeño
shortcut-new-document = Nuevo capítulo
shortcut-blank = Nuevo documento en blanco
shortcut-open = Abrir archivo
shortcut-save = Guardar
shortcut-save-as = Guardar como
shortcut-undo = Deshacer / Rehacer
shortcut-find = Buscar / Reemplazar
shortcut-search = Buscar en todo el proyecto
shortcut-delete-line = Borrar la línea
shortcut-page = Avanzar o retroceder una página
shortcut-lint = Revisar / Formatear Markdown
shortcut-help = Mostrar / ocultar los atajos
shortcut-quit = Salir

## Sidebar

sidebar-home = Inicio
    .hint = Inicio del proyecto
sidebar-open = Abrir…
    .hint = Abrir otro proyecto
sidebar-outline = Esquema
sidebar-notes = Notas
sidebar-add-research = Añadir archivos de investigación
sidebar-all-files = Todos los archivos
sidebar-folder-unavailable = Esta carpeta no está disponible.
create-file-failed = No se pudo crear el archivo: { $error }

## Status bar and window title

status-edited = Editado
    .autosave = Se guarda solo cuando hace una pausa
    .unsaved = Cambios sin guardar (Ctrl+S para guardar)
status-saved = Guardado
status-not-saved = Aún sin guardar
status-goal = Meta de la sesión: { $words } de { $goal } { $count ->
    [one] palabra
   *[other] palabras
}
status-session = { $words } en esta sesión
    .hint = Las palabras de la sesión cuentan solo lo que escribe aquí
status-manuscript = { $words } en el manuscrito
status-words-in = { $count ->
    [one] { $words } palabra
   *[other] { $words } palabras
} { $gender ->
    [feminine] en la
   *[other] en el
} { $document }
status-reading = { $minutes } min de lectura
status-selected = { $words } seleccionadas
status-line = Lín { $line }
title-home = Inicio
title-untitled = Sin título

## Home screen

home-tagline = Un lugar tranquilo para historias largas.
home-current-project = Proyecto actual
home-continue = Seguir escribiendo  «{ $title }»
home-new-project = Nuevo Proyecto…
home-open-project = Abrir Proyecto…
home-blank-page = Página en Blanco
home-recent = Proyectos recientes
home-keys = F11 modo concentración   ·   Ctrl+B barra lateral   ·   Ctrl+H todos los atajos

## Focus mode

focus-goal = { $words } / { $goal } en esta sesión
focus-unsaved = sin guardar
focus-hint = Modo concentración — pulse F11 o Esc para salir

## Buttons used in several places

button-save = Guardar
button-discard = Descartar
button-cancel = Cancelar
button-close = Cerrar
button-create = Crear
button-save-and-exit = Guardar y Salir
button-discard-and-exit = Descartar y Salir

## Unsaved changes

discard-title = Cambios Sin Guardar
discard-exit = Hay cambios sin guardar. ¿Guardar antes de salir?
discard-new-document = Hay cambios sin guardar. ¿Guardar antes de empezar un documento nuevo?
discard-home = Hay cambios sin guardar. ¿Guardar antes de ir al inicio del proyecto?
discard-open-file = Hay cambios sin guardar. ¿Guardar antes de abrir otro archivo?
discard-create-project = Hay cambios sin guardar. ¿Guardar antes de crear el proyecto { $path }?
discard-open-project = Hay cambios sin guardar. ¿Guardar antes de abrir el proyecto { $path }?
discard-open = Hay cambios sin guardar. ¿Guardar antes de abrir { $name }?

## File changed on disk

conflict-title = Archivo Modificado en el Disco
conflict-this-file = Este archivo
conflict-changed = { $name } se modificó fuera de Quillwright (por ejemplo, por un agente) mientras tenía cambios sin guardar.
conflict-deleted = { $name } se borró o se movió fuera de Quillwright.
conflict-unreadable = Ya no se puede leer { $name }: { $error }
conflict-reload = Recargar del Disco
    .hint = Descarta sus cambios sin guardar
conflict-keep = Conservar Mi Versión
    .hint = Sigue editando; al guardar se sobrescribirá la versión del disco
watch-degraded = El seguimiento de archivos está limitado: { $warning }

## Opening and saving

dialog-open-project = Abrir Proyecto
dialog-choose-parent = Elegir la Carpeta
cannot-read = No se pudo leer { $path }: { $error }
open-failed = No se pudo abrir { $path }: { $error }
save-failed = Error al guardar: { $error }
project-create-failed = No se pudo crear el proyecto { $path }: { $error }
project-open-failed = No se pudo abrir el proyecto { $path }: { $error }

## New Project

new-project-title = Nuevo Proyecto
new-project-name = Nombre de la carpeta del proyecto
new-project-parent = Crear dentro de
new-project-choose = Elegir Carpeta...
new-project-kind = Tipo de proyecto
new-project-language = Idioma de la plantilla
new-project-creates = Crea { $files }.
new-project-note = Las plantillas son opcionales. Las carpetas existentes no se sobrescribirán.

## Find and replace

find-title = Buscar / Reemplazar
find-next = Siguiente
find-previous = Anterior
find-replace = Reemplazar
find-replace-all = Reemplazar Todo
find-found = Coincidencia encontrada
find-none = No se encontró nada
find-no-previous = No hay coincidencia anterior
find-empty = Escriba el texto que busca
find-select = Seleccione una coincidencia o use Buscar
find-mismatch = La selección no coincide con el texto buscado
find-replaced = Selección reemplazada
find-nothing-to-replace = No hay nada que reemplazar
find-replaced-all = { $count ->
    [one] { $count } coincidencia reemplazada
   *[other] { $count } coincidencias reemplazadas
}
line-deleted = Línea borrada

## Lint and format tools (Tools menu)

tool-output = Salida de la Herramienta
tool-output-none = No hay salida.
tool-no-lint = No hay ningún comando de revisión configurado. Añada una entrada lint en [tools] en config.toml.
tool-no-format = No hay ningún comando de formato configurado. Añada una entrada format en [tools] en config.toml.
tool-empty = El comando configurado está vacío.
tool-save-first = Guarde antes de usar esta herramienta en el archivo actual, o desactive *_use_open_file.
tool-not-a-file = La ruta actual '{ $path }' no es un archivo.
tool-no-file = No hay ningún archivo abierto. Guarde primero el documento o desactive *_use_open_file.
tool-temp-failed = No se pudo preparar el archivo temporal: { $error }
tool-run-failed = No se pudo ejecutar '{ $tool }': { $error }
project-name-invalid = Escriba un nombre de carpeta, sin separadores de ruta.
project-name-single = Use un solo nombre de carpeta.
project-parent-missing = Elija una carpeta que ya exista.
button-copy = Copiar

## Assistant panel

assistant-title = Asistente
assistant-done = Listo
assistant-settings = Ajustes
assistant-intro = Quillwright usa una herramienta de IA instalada en su equipo y muestra aquí la respuesta. El manuscrito solo cambia cuando acepta una sugerencia.
assistant-choose-tool = Elija una herramienta
assistant-command = Comando
assistant-prompt-argument = Pasar el prompt como último argumento (predeterminado: stdin)
assistant-command-note = El comando se ejecuta en la carpeta del proyecto. El inicio de sesión y las claves de API quedan en la herramienta.
preset-claude-note = Usa su sesión de Claude Code. Instalación: https://claude.com/claude-code
preset-codex-note = Usa su sesión de Codex CLI, en un entorno aislado de solo lectura.
preset-ollama-note = Ejecuta un modelo local; el manuscrito nunca sale de su equipo.
assistant-open-document = { $gender ->
    [feminine] Abra una { $document } para usar el asistente.
   *[other] Abra un { $document } para usar el asistente.
}
assistant-selected = { $count ->
    [one] { $words } palabra seleccionada
   *[other] { $words } palabras seleccionadas
}
assistant-cursor = cursor en la línea { $line }
assistant-needs-selection = Seleccione primero un pasaje
assistant-ask = Preguntar
assistant-sends = Envía { $what } a `{ $tool }`.
assistant-this-document = { $gender ->
    [feminine] esta { $document }
   *[other] este { $document }
}
assistant-sent-passages = { $count ->
    [one] { $count } pasaje de referencia
   *[other] { $count } pasajes de referencia
}
assistant-passages = Pasajes de referencia
assistant-clear = Quitar todos
assistant-remove-passage = Dejar de enviar este pasaje
assistant-writing = Escribiendo…
assistant-details = Detalles
assistant-prompt-sent = Prompt enviado
assistant-revision = Revisión sugerida
assistant-continuation = Continuación sugerida
assistant-answer = Respuesta
assistant-replace = Reemplazar la selección
assistant-insert = Insertar en el cursor
assistant-retry = Intentar de nuevo
assistant-other-file = Esta respuesta es de otro archivo; ábralo de nuevo para aplicarla.
assistant-empty-answer = El asistente devolvió una respuesta vacía.
assistant-select-first = Seleccione primero un pasaje en el editor.
assistant-start-failed = No se pudo iniciar `{ $tool }`: { $error }.
assistant-not-found = ¿Está instalado y en su PATH? Revise los ajustes del asistente.
assistant-passage-changed = El pasaje seleccionado cambió desde la petición. Copie la sugerencia en su lugar.
job-lost = Se perdió el proceso
job-failed = El comando falló ({ $status })

## Autosave and version history

autosave-failed = El guardado automático falló: { $error }
history-copy-failed = No se pudo guardar una copia en el historial: { $error }
history-title = Historial de Versiones — { $title }
history-read-failed = No se pudo leer esta copia: { $error }
history-empty = Todavía no hay versiones anteriores de este archivo.
    .when = Se guarda una copia cuando un guardado reemplaza el archivo (como mucho cada 10 minutos, y siempre antes de un recorte grande) y cada vez que el cambio de un agente reemplaza el texto del editor.
history-compared = Comparado con el texto del editor: el texto tachado solo está en el editor; el resaltado solo está en esta versión.
history-restore = Restaurar Esta Versión
    .hint = Reemplaza el texto del editor. Antes se guarda el texto actual en el historial.
history-restored = Se restauró una versión anterior
history-today = Hoy { $time }
history-yesterday = Ayer { $time }
history-this-year = %d/%m %H:%M
history-older = %d/%m/%Y %H:%M

## Spelling

spelling-none = No hay sugerencias de ortografía
spelling-no-suggestions = No hay sugerencias
spelling-add = Añadir al Diccionario del Proyecto
    .hint = Añade «{ $word }» a { $file } en la carpeta del proyecto
spelling-add-failed = No se pudo actualizar { $file }: { $error }
spelling-ignore = Ignorar en Esta Sesión
spelling-check = Revisar Ortografía
    .hint = Subraya las palabras mal escritas; haga clic derecho en una para ver sugerencias
spelling-language = Idioma de la Ortografía
spelling-language-book = Este libro (se guarda en metadata.yaml)
spelling-language-default = Predeterminado para libros sin idioma
spelling-loading = Cargando { $language }…
spelling-checking = Revisando { $language }
spelling-folder = Abrir la Carpeta de Diccionarios
    .hint = Ponga aquí archivos .aff y .dic de Hunspell para más idiomas:
        { $path }
spelling-no-dictionary = No hay diccionario para { $language }
spelling-load-failed = No se pudo cargar el diccionario { $language }
write-failed = No se pudo escribir { $path }

## Project search (sidebar)

search-hint = Buscar en el proyecto
search-searching = Buscando…
search-none = Sin resultados
search-results = { $count ->
    [one] 1 resultado
   *[other] { $files ->
        [one] { $count } resultados en 1 archivo
       *[other] { $count } resultados en { $files } archivos
    }
}
search-reading = leyendo { $title } por primera vez…
search-more = buscando…
search-failed = { $count ->
    [one] { $count } archivo no se pudo leer
   *[other] { $count } archivos no se pudieron leer
}
search-and-more = …y { $count } más
search-note = Nota
search-page = p. { $page }

## Reference panel (research files)

dialog-add-research = Añadir Archivos de Investigación
reference-unreadable = No se pudo leer el archivo.
reference-open-failed = No se pudo abrir este archivo: { $error }
reference-missing = Este archivo se movió o se borró.
reference-add-failed = No se pudieron añadir los archivos de investigación: { $error }
reference-read-only = solo lectura
reference-edit = Editar
    .hint = Abre este archivo en el editor
reference-actual-size = Tamaño Real
reference-fit-width = Ajustar al Ancho
reference-system-viewer = Abrir con el Visor del Sistema
reference-use = Usar como Contexto
    .hint = Envía el pasaje seleccionado, con su fuente, al asistente
reference-click-actual = Haga clic para ver el tamaño real
reference-click-fit = Haga clic para ajustar al panel
reference-line = línea { $line }
reference-lines = líneas { $first }–{ $last }
reference-unsupported = Quillwright no puede mostrar este tipo de archivo.
reference-cut = el resto de este archivo no se muestra
file-text = Texto
file-notes = Notas
file-image = Imagen
file-pdf = PDF
file-other = Archivo

## PDF viewer

pdf-unreadable = No se pudo leer el PDF.
pdf-no-pages = Este PDF no tiene páginas.
pdf-password = Este PDF está protegido con contraseña, y Quillwright todavía no puede abrirlo.
pdf-invalid = Este archivo no parece un PDF que Quillwright pueda leer.
pdf-page-failed = No se pudo mostrar esta página.
pdf-page-number = Página { $page }
pdf-pages = pp. { $first }–{ $last }
pdf-no-text = Este PDF no tiene texto en el que buscar.
pdf-reading = leyendo { $percent } %
pdf-no-matches = Sin coincidencias
pdf-match-of = { $current } de { $total }
pdf-matches = { $count ->
    [one] { $count } coincidencia
   *[other] { $count } coincidencias
}
pdf-page = Página
    .hint = Escriba un número de página, o arrastre
pdf-page-count = de { $count }
pdf-zoom-in = Acercar
pdf-zoom-fit = Ajustar la página al panel
pdf-zoom-out = Alejar
pdf-find = Buscar en este PDF
pdf-previous = Coincidencia anterior
pdf-next = Coincidencia siguiente

## Export Manuscript

export-title = Exportar Manuscrito
export-book-title = Título
export-author = Autor
export-language = Idioma
    .hint = Se usa para la división de palabras y en los lectores de libros electrónicos. Vacío: el lang de metadata.yaml, o inglés.
export-page-size = Tamaño de página
export-toc = Índice
export-summary = { $documents } · { $words }, en el orden de la barra lateral
export-uses = Usa { $file }
export-styles = Estilos de { $file }
export-running = Exportando…
export-start = Exportar…
export-nothing = Todavía no hay nada que exportar.
export-done = Se exportó { $file }
export-open = Abrir
export-show = Mostrar en la Carpeta
export-save-failed = No se pudo guardar { $path }: { $error }
export-stopped = La exportación se detuvo de forma inesperada.
export-metadata-ignored = Se ignoró { $file }: { $error }
export-write-failed = No se pudo escribir el archivo: { $error }
export-no-styles = { $file } no tiene estilos y se ignoró.
export-image-skipped = Imagen no incluida: { $file }
format-docx = Word (.docx)
    .hint = Para editores, agentes y Google Docs
format-epub = Libro electrónico EPUB (.epub)
    .hint = Para lectores de libros electrónicos y lectores beta
format-pdf = PDF (.pdf)
    .hint = Para imprimir y compartir con un diseño fijo
format-odt = OpenDocument (.odt)
    .hint = Para LibreOffice
format-html = Página web (.html)
    .hint = Una sola página, completa en sí misma
page-a5 = A5 (148 × 210 mm)
page-a4 = A4 (210 × 297 mm)
page-letter = Carta EE. UU. (8,5 × 11 pulg.)
page-trade = Rústica comercial (6 × 9 pulg.)

## Themes (View → Theme)

theme-paper = Papel
theme-sepia = Sepia
theme-night = Noche
reference-loading = Cargando…
