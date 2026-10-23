# Textos da interface do Quillwright em português do Brasil.
# O original é ../en/quillwright.ftl, que explica as variáveis. Mantenha os mesmos ids.

## Counts

words = { $count ->
    [one] { $words } palavra
   *[other] { $words } palavras
}
thousands = { $count } mil

## Menus

menu-file = Arquivo
menu-view = Exibir
menu-assistant = Assistente
menu-tools = Ferramentas
menu-help = Ajuda
menu-lint = Verificar Markdown
menu-format = Formatar Markdown
menu-shortcuts = Atalhos de Teclado
menu-add-research = Adicionar Arquivos de Pesquisa...
    .hint = Copia PDFs, imagens e notas para a pasta de pesquisa do projeto
menu-new-project = Novo Projeto...
menu-open-project = Abrir Projeto...
menu-project-home = Início do Projeto
menu-new-blank = Novo Documento em Branco
menu-open-file = Abrir Arquivo...
menu-save = Salvar
menu-save-as = Salvar Como...
menu-autosave = Salvar automaticamente
    .hint = Salva o arquivo aberto logo depois que você para de digitar
menu-history = Histórico de Versões...
    .hint = Versões anteriores deste arquivo, guardadas neste computador
menu-export = Exportar Manuscrito...
menu-exit = Sair
menu-assistant-panel = Painel do Assistente
menu-assistant-settings = Configurações do Assistente...
menu-theme = Tema
menu-theme-system = Igual ao sistema
menu-focus = Modo Foco
menu-sidebar = Barra Lateral
menu-search = Pesquisar no Projeto...
menu-reference = Painel de Referência
    .hint = O arquivo de pesquisa aberto por último
menu-typewriter = Rolagem de Máquina de Escrever
    .hint = Mantém a linha que você está escrevendo no meio da tela
menu-dim = Esmaecer Outros Parágrafos
    .hint = Esmaece tudo, exceto o parágrafo que você está escrevendo
menu-line-numbers = Números de Linha
menu-larger = Texto Maior
menu-smaller = Texto Menor
menu-default-size = Tamanho Padrão do Texto
menu-session-goal = Meta da sessão (palavras)
    .hint = 0 esconde a meta
menu-language = Idioma da Interface
menu-language-system = Igual ao sistema

## Keyboard shortcuts (Help → Keyboard Shortcuts)

shortcuts-title = Atalhos de Teclado
shortcut-focus = Modo foco (Esc para sair)
shortcut-sidebar = Mostrar / esconder a barra lateral
shortcut-assistant = Painel do assistente de IA
shortcut-export = Exportar o manuscrito
shortcut-send = Enviar o pedido ao assistente
shortcut-typewriter = Rolagem de máquina de escrever
shortcut-dim = Esmaecer outros parágrafos
shortcut-text-size = Texto maior / menor
shortcut-new-document = Novo capítulo
shortcut-blank = Novo documento em branco
shortcut-open = Abrir arquivo
shortcut-save = Salvar
shortcut-save-as = Salvar como
shortcut-undo = Desfazer / Refazer
shortcut-find = Localizar / Substituir
shortcut-search = Pesquisar no projeto inteiro
shortcut-delete-line = Apagar a linha
shortcut-page = Avançar ou voltar uma página
shortcut-lint = Verificar / Formatar Markdown
shortcut-help = Mostrar / esconder os atalhos
shortcut-quit = Sair

## Sidebar

sidebar-home = Início
    .hint = Início do projeto
sidebar-open = Abrir…
    .hint = Abrir outro projeto
sidebar-outline = Estrutura
sidebar-notes = Notas
sidebar-add-research = Adicionar arquivos de pesquisa
sidebar-all-files = Todos os arquivos
sidebar-folder-unavailable = Esta pasta não está disponível.
create-file-failed = Não foi possível criar o arquivo: { $error }

## Status bar and window title

status-edited = Editado
    .autosave = Salva sozinho quando você faz uma pausa
    .unsaved = Alterações não salvas (Ctrl+S para salvar)
status-saved = Salvo
status-not-saved = Ainda não salvo
status-goal = Meta da sessão: { $words } de { $goal } { $count ->
    [one] palavra
   *[other] palavras
}
status-session = { $words } nesta sessão
    .hint = As palavras da sessão contam só o que você digita aqui
status-manuscript = { $words } no manuscrito
status-words-in = { $count ->
    [one] { $words } palavra
   *[other] { $words } palavras
} { $gender ->
    [feminine] na
   *[other] no
} { $document }
status-reading = { $minutes } min de leitura
status-selected = { $words } selecionadas
status-line = Lin { $line }
title-home = Início
title-untitled = Sem título

## Home screen

home-tagline = Um lugar tranquilo para histórias longas.
home-current-project = Projeto atual
home-continue = Continuar escrevendo  “{ $title }”
home-new-project = Novo Projeto…
home-open-project = Abrir Projeto…
home-blank-page = Página em Branco
home-recent = Projetos recentes
home-keys = F11 modo foco   ·   Ctrl+B barra lateral   ·   Ctrl+H todos os atalhos

## Focus mode

focus-goal = { $words } / { $goal } nesta sessão
focus-unsaved = não salvo
focus-hint = Modo foco — pressione F11 ou Esc para sair

## Buttons used in several places

button-save = Salvar
button-discard = Descartar
button-cancel = Cancelar
button-close = Fechar
button-create = Criar
button-save-and-exit = Salvar e Sair
button-discard-and-exit = Descartar e Sair

## Unsaved changes

discard-title = Alterações Não Salvas
discard-exit = Há alterações não salvas. Salvar antes de sair?
discard-new-document = Há alterações não salvas. Salvar antes de começar um novo documento?
discard-home = Há alterações não salvas. Salvar antes de ir para o início do projeto?
discard-open-file = Há alterações não salvas. Salvar antes de abrir outro arquivo?
discard-create-project = Há alterações não salvas. Salvar antes de criar o projeto { $path }?
discard-open-project = Há alterações não salvas. Salvar antes de abrir o projeto { $path }?
discard-open = Há alterações não salvas. Salvar antes de abrir { $name }?

## File changed on disk

conflict-title = Arquivo Alterado no Disco
conflict-this-file = Este arquivo
conflict-changed = { $name } foi alterado fora do Quillwright (por exemplo, por um agente) enquanto você tinha edições não salvas.
conflict-deleted = { $name } foi apagado ou movido fora do Quillwright.
conflict-unreadable = Não é mais possível ler { $name }: { $error }
conflict-reload = Recarregar do Disco
    .hint = Descarta as suas edições não salvas
conflict-keep = Manter a Minha Versão
    .hint = Continua editando; ao salvar, a versão do disco será substituída
watch-degraded = O acompanhamento de arquivos está limitado: { $warning }

## Opening and saving

dialog-open-project = Abrir Projeto
dialog-choose-parent = Escolher a Pasta
cannot-read = Não foi possível ler { $path }: { $error }
open-failed = Não foi possível abrir { $path }: { $error }
save-failed = Erro ao salvar: { $error }
project-create-failed = Não foi possível criar o projeto { $path }: { $error }
project-open-failed = Não foi possível abrir o projeto { $path }: { $error }

## New Project

new-project-title = Novo Projeto
new-project-name = Nome da pasta do projeto
new-project-parent = Criar dentro de
new-project-choose = Escolher Pasta...
new-project-kind = Tipo de projeto
new-project-language = Idioma do modelo
new-project-creates = Cria { $files }.
new-project-note = Os modelos são opcionais. Pastas existentes não serão substituídas.

## Find and replace

find-title = Localizar / Substituir
find-next = Próximo
find-previous = Anterior
find-replace = Substituir
find-replace-all = Substituir Tudo
find-found = Encontrado
find-none = Nada encontrado
find-no-previous = Nenhuma ocorrência anterior
find-empty = Digite o texto a localizar
find-select = Selecione uma ocorrência ou use Localizar
find-mismatch = A seleção não corresponde ao texto procurado
find-replaced = Seleção substituída
find-nothing-to-replace = Nada para substituir
find-replaced-all = { $count ->
    [one] { $count } ocorrência substituída
   *[other] { $count } ocorrências substituídas
}
line-deleted = Linha apagada

## Lint and format tools (Tools menu)

tool-output = Saída da Ferramenta
tool-output-none = Nenhuma saída.
tool-no-lint = Nenhum comando de verificação configurado. Adicione uma entrada lint em [tools] no config.toml.
tool-no-format = Nenhum comando de formatação configurado. Adicione uma entrada format em [tools] no config.toml.
tool-empty = O comando configurado está vazio.
tool-save-first = Salve antes de usar esta ferramenta no arquivo atual, ou desative *_use_open_file.
tool-not-a-file = O caminho atual '{ $path }' não é um arquivo.
tool-no-file = Nenhum arquivo aberto. Salve o documento primeiro ou desative *_use_open_file.
tool-temp-failed = Não foi possível preparar o arquivo temporário: { $error }
tool-run-failed = Não foi possível executar '{ $tool }': { $error }
project-name-invalid = Digite um nome de pasta, sem separadores de caminho.
project-name-single = Use um único nome de pasta.
project-parent-missing = Escolha uma pasta que já existe.
button-copy = Copiar

## Assistant panel

assistant-title = Assistente
assistant-done = Pronto
assistant-settings = Configurações
assistant-intro = O Quillwright usa uma ferramenta de IA instalada no seu computador e mostra a resposta aqui. O manuscrito só muda quando você aceita uma sugestão.
assistant-choose-tool = Escolha uma ferramenta
assistant-command = Comando
assistant-prompt-argument = Passar o prompt como último argumento (padrão: stdin)
assistant-command-note = O comando roda na pasta do projeto. Login e chaves de API ficam com a ferramenta.
preset-claude-note = Usa o seu login do Claude Code. Instalação: https://claude.com/claude-code
preset-codex-note = Usa o seu login do Codex CLI, em uma sandbox somente leitura.
preset-ollama-note = Roda um modelo local; o manuscrito nunca sai do seu computador.
assistant-open-document = { $gender ->
    [feminine] Abra uma { $document } para usar o assistente.
   *[other] Abra um { $document } para usar o assistente.
}
assistant-selected = { $count ->
    [one] { $words } palavra selecionada
   *[other] { $words } palavras selecionadas
}
assistant-cursor = cursor na linha { $line }
assistant-needs-selection = Selecione um trecho primeiro
assistant-ask = Perguntar
assistant-sends = Envia { $what } para `{ $tool }`.
assistant-this-document = { $gender ->
    [feminine] esta { $document }
   *[other] este { $document }
}
assistant-sent-passages = { $count ->
    [one] { $count } trecho de referência
   *[other] { $count } trechos de referência
}
assistant-passages = Trechos de referência
assistant-clear = Limpar
assistant-remove-passage = Parar de enviar este trecho
assistant-writing = Escrevendo…
assistant-details = Detalhes
assistant-prompt-sent = Prompt enviado
assistant-revision = Revisão sugerida
assistant-continuation = Continuação sugerida
assistant-answer = Resposta
assistant-replace = Substituir a seleção
assistant-insert = Inserir no cursor
assistant-retry = Tentar de novo
assistant-other-file = Esta resposta é de outro arquivo; abra-o de novo para aplicar.
assistant-empty-answer = O assistente devolveu uma resposta vazia.
assistant-select-first = Selecione primeiro um trecho no editor.
assistant-start-failed = Não foi possível iniciar `{ $tool }`: { $error }.
assistant-not-found = Ele está instalado e no seu PATH? Confira as configurações do assistente.
assistant-passage-changed = O trecho selecionado mudou desde o pedido. Copie a sugestão em vez disso.
job-lost = O processo se perdeu
job-failed = O comando falhou ({ $status })

## Autosave and version history

autosave-failed = O salvamento automático falhou: { $error }
history-copy-failed = Não foi possível guardar uma cópia no histórico: { $error }
history-title = Histórico de Versões — { $title }
history-read-failed = Não foi possível ler esta cópia: { $error }
history-empty = Ainda não há versões anteriores deste arquivo.
    .when = Uma cópia é guardada quando um salvamento substitui o arquivo (no máximo a cada 10 minutos, e sempre antes de um corte grande) e sempre que a alteração de um agente substitui o texto no editor.
history-compared = Comparado com o texto no editor: o texto riscado está só no editor; o destacado está só nesta versão.
history-restore = Restaurar Esta Versão
    .hint = Substitui o texto do editor. O texto atual é guardado no histórico antes.
history-restored = Uma versão anterior foi restaurada
history-today = Hoje { $time }
history-yesterday = Ontem { $time }
history-this-year = %d/%m %H:%M
history-older = %d/%m/%Y %H:%M

## Spelling

spelling-none = Nenhuma sugestão de ortografia
spelling-no-suggestions = Nenhuma sugestão
spelling-add = Adicionar ao Dicionário do Projeto
    .hint = Adiciona “{ $word }” a { $file } na pasta do projeto
spelling-add-failed = Não foi possível atualizar { $file }: { $error }
spelling-ignore = Ignorar Nesta Sessão
spelling-check = Verificar Ortografia
    .hint = Sublinha palavras com erro; clique com o botão direito em uma para ver sugestões
spelling-language = Idioma da Ortografia
spelling-language-book = Este livro (salvo em metadata.yaml)
spelling-language-default = Padrão para livros sem idioma
spelling-loading = Carregando { $language }…
spelling-checking = Verificando { $language }
spelling-folder = Abrir a Pasta de Dicionários
    .hint = Coloque aqui arquivos .aff e .dic do Hunspell para mais idiomas:
        { $path }
spelling-no-dictionary = Nenhum dicionário para { $language }
spelling-load-failed = Não foi possível carregar o dicionário { $language }
write-failed = Não foi possível gravar { $path }

## Project search (sidebar)

search-hint = Pesquisar no projeto
search-searching = Pesquisando…
search-none = Nenhum resultado
search-results = { $count ->
    [one] 1 resultado
   *[other] { $files ->
        [one] { $count } resultados em 1 arquivo
       *[other] { $count } resultados em { $files } arquivos
    }
}
search-reading = lendo { $title } pela primeira vez…
search-more = pesquisando…
search-failed = { $count ->
    [one] { $count } arquivo não pôde ser lido
   *[other] { $count } arquivos não puderam ser lidos
}
search-and-more = …e mais { $count }
search-note = Nota
search-page = p. { $page }

## Reference panel (research files)

dialog-add-research = Adicionar Arquivos de Pesquisa
reference-unreadable = Não foi possível ler o arquivo.
reference-open-failed = Não foi possível abrir este arquivo: { $error }
reference-missing = Este arquivo foi movido ou apagado.
reference-add-failed = Não foi possível adicionar os arquivos de pesquisa: { $error }
reference-read-only = somente leitura
reference-edit = Editar
    .hint = Abre este arquivo no editor
reference-actual-size = Tamanho Real
reference-fit-width = Ajustar à Largura
reference-system-viewer = Abrir no Visualizador do Sistema
reference-use = Usar como Contexto
    .hint = Envia o trecho selecionado, com a fonte, para o assistente
reference-click-actual = Clique para ver no tamanho real
reference-click-fit = Clique para ajustar ao painel
reference-line = linha { $line }
reference-lines = linhas { $first }–{ $last }
reference-unsupported = O Quillwright não consegue mostrar este tipo de arquivo.
reference-cut = o restante deste arquivo não é mostrado
file-text = Texto
file-notes = Notas
file-image = Imagem
file-pdf = PDF
file-other = Arquivo

## PDF viewer

pdf-unreadable = Não foi possível ler o PDF.
pdf-no-pages = Este PDF não tem páginas.
pdf-password = Este PDF é protegido por senha, e o Quillwright ainda não consegue abri-lo.
pdf-invalid = Este arquivo não parece um PDF que o Quillwright consiga ler.
pdf-page-failed = Não foi possível mostrar esta página.
pdf-page-number = Página { $page }
pdf-pages = pp. { $first }–{ $last }
pdf-no-text = Este PDF não tem texto para pesquisar.
pdf-reading = lendo { $percent }%
pdf-no-matches = Nada encontrado
pdf-match-of = { $current } de { $total }
pdf-matches = { $count ->
    [one] { $count } ocorrência
   *[other] { $count } ocorrências
}
pdf-page = Página
    .hint = Digite o número de uma página, ou arraste
pdf-page-count = de { $count }
pdf-zoom-in = Aumentar o zoom
pdf-zoom-fit = Ajustar a página ao painel
pdf-zoom-out = Diminuir o zoom
pdf-find = Localizar neste PDF
pdf-previous = Ocorrência anterior
pdf-next = Próxima ocorrência

## Export Manuscript

export-title = Exportar Manuscrito
export-book-title = Título
export-author = Autor
export-language = Idioma
    .hint = Usado na hifenização e pelos leitores de e-book. Vazio: o lang do metadata.yaml, ou inglês.
export-page-size = Tamanho da página
export-toc = Sumário
export-summary = { $documents } · { $words }, na ordem da barra lateral
export-uses = Usa { $file }
export-styles = Estilos de { $file }
export-running = Exportando…
export-start = Exportar…
export-nothing = Ainda não há nada para exportar.
export-done = { $file } exportado
export-open = Abrir
export-show = Mostrar na Pasta
export-save-failed = Não foi possível salvar { $path }: { $error }
export-stopped = A exportação parou de forma inesperada.
export-metadata-ignored = { $file } foi ignorado: { $error }
export-write-failed = Não foi possível gravar o arquivo: { $error }
export-no-styles = { $file } não tem estilos e foi ignorado.
export-image-skipped = Imagem não incluída: { $file }
format-docx = Word (.docx)
    .hint = Para editores, agentes e Google Docs
format-epub = E-book EPUB (.epub)
    .hint = Para leitores de e-book e leitores beta
format-pdf = PDF (.pdf)
    .hint = Para imprimir e compartilhar com layout fixo
format-odt = OpenDocument (.odt)
    .hint = Para o LibreOffice
format-html = Página web (.html)
    .hint = Uma única página, completa em si mesma
page-a5 = A5 (148 × 210 mm)
page-a4 = A4 (210 × 297 mm)
page-letter = Carta americana (8,5 × 11 pol.)
page-trade = Brochura comercial (6 × 9 pol.)

## Themes (View → Theme)

theme-paper = Papel
theme-sepia = Sépia
theme-night = Noite
reference-loading = Carregando…
