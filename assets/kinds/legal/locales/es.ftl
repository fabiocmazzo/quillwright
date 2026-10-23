# El tipo Caso jurídico tal como lo muestra la aplicación. Vea ../../novel/locales/en.ftl
# para qué es cada mensaje.

legal = Caso jurídico
legal-description = Los escritos de un caso, con sus hechos, cronología, partes y pruebas

legal-research = Expediente
legal-research-hint = Expediente judicial, documentos y otros PDF — pulse + para añadir.

legal-documents = Escritos
legal-documents-one = escrito
    .gender = masculine
legal-documents-count = { $count ->
    [one] { $count } escrito
   *[other] { $count } escritos
}
legal-documents-new = Nuevo escrito
legal-documents-new-menu = Nuevo Escrito
legal-documents-empty = Todavía no hay escritos — pulse + para empezar uno.
legal-documents-first = Redactar el primer escrito

legal-sheets = Partes
legal-sheets-one = parte
    .gender = feminine
legal-sheets-count = { $count ->
    [one] { $count } parte
   *[other] { $count } partes
}
legal-sheets-new = Nueva parte
legal-sheets-new-menu = Nueva Parte
legal-sheets-empty = Todavía no hay partes — pulse + para añadir una.
legal-sheets-first = Añadir la primera parte
legal-sheets-sent = { $count ->
    [one] { $count } ficha de parte
   *[other] { $count } fichas de partes
}

legal-ask-hint = Pregunte lo que quiera, o guíe los botones de arriba («más formal», «cite las pruebas»)

legal-action-rewrite = Reescribir la selección
    .hint = Reescribe el pasaje seleccionado
legal-action-draft = Redactar a partir de los hechos
    .hint = Redacta el texto en el cursor a partir de los hechos y las pruebas
legal-action-check = Contrastar con los hechos
    .hint = Compara este escrito con los hechos, la cronología y las pruebas
legal-action-questions = Preguntas para el cliente
    .hint = Lo que aún se desconoce, como preguntas para el cliente

legal-context-facts = hechos
legal-context-timeline = cronología
legal-context-evidence = lista de pruebas
legal-context-client-questions = preguntas al cliente
