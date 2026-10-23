# O tipo Processo como o app o mostra. Veja ../../novel/locales/en.ftl para o que é cada
# mensagem.

legal = Processo jurídico
legal-description = As peças de um processo, com os fatos, a cronologia, as partes e as provas

legal-research = Autos
legal-research-hint = Autos, documentos e outros PDFs — pressione + para adicionar.

legal-documents = Peças
legal-documents-one = peça
    .gender = feminine
legal-documents-count = { $count ->
    [one] { $count } peça
   *[other] { $count } peças
}
legal-documents-new = Nova peça
legal-documents-new-menu = Nova Peça
legal-documents-empty = Ainda não há peças — pressione + para começar uma.
legal-documents-first = Escrever a primeira peça

legal-sheets = Partes
legal-sheets-one = parte
    .gender = feminine
legal-sheets-count = { $count ->
    [one] { $count } parte
   *[other] { $count } partes
}
legal-sheets-new = Nova parte
legal-sheets-new-menu = Nova Parte
legal-sheets-empty = Ainda não há partes — pressione + para adicionar uma.
legal-sheets-first = Adicionar a primeira parte
legal-sheets-sent = { $count ->
    [one] { $count } ficha de parte
   *[other] { $count } fichas de partes
}

legal-ask-hint = Pergunte qualquer coisa, ou oriente os botões acima (“mais formal”, “cite os documentos”)

legal-action-rewrite = Reescrever a seleção
    .hint = Reescreve o trecho selecionado
legal-action-draft = Redigir a partir dos fatos
    .hint = Redige o texto no cursor a partir dos fatos e das provas
legal-action-check = Conferir com os fatos
    .hint = Compara esta peça com os fatos, a cronologia e as provas
legal-action-questions = Perguntas para o cliente
    .hint = O que ainda não se sabe, como perguntas para o cliente

legal-context-facts = fatos
legal-context-timeline = cronologia
legal-context-evidence = lista de provas
legal-context-client-questions = perguntas ao cliente
