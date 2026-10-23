# El tipo Novela tal como lo muestra la aplicación. Vea en.ftl para qué es cada mensaje.

novel = Novela
novel-description = Capítulos, fichas de personajes y las reglas de su mundo

novel-research = Investigación
novel-research-hint = PDF, imágenes y notas — pulse + para añadir.

novel-documents = Capítulos
novel-documents-one = capítulo
    .gender = masculine
novel-documents-count = { $count ->
    [one] { $count } capítulo
   *[other] { $count } capítulos
}
novel-documents-new = Nuevo capítulo
novel-documents-new-menu = Nuevo Capítulo
novel-documents-empty = Todavía no hay capítulos — pulse + para empezar uno.
novel-documents-first = Escribir el primer capítulo

novel-sheets = Personajes
novel-sheets-one = personaje
    .gender = masculine
novel-sheets-count = { $count ->
    [one] { $count } personaje
   *[other] { $count } personajes
}
novel-sheets-new = Nuevo personaje
novel-sheets-new-menu = Nuevo Personaje
novel-sheets-empty = Todavía no hay personajes — pulse + para añadir uno.
novel-sheets-first = Añadir el primer personaje
novel-sheets-sent = { $count ->
    [one] { $count } ficha de personaje
   *[other] { $count } fichas de personajes
}

novel-ask-hint = Pregunte lo que quiera, o guíe los botones de arriba («más tensión, frases más cortas»)

novel-action-rewrite = Reescribir la selección
    .hint = Reescribe el pasaje seleccionado
novel-action-continue = Continuar la escena
    .hint = Escribe lo que sigue, en el cursor
novel-action-alternatives = Tres alternativas
    .hint = Tres versiones distintas de la selección
novel-action-continuity = Revisar la continuidad
    .hint = Revisa este capítulo frente al canon

novel-context-world-rules = reglas del mundo
