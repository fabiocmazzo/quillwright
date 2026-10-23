# The Novel kind as the app shows it. Each message starts with the kind's id ("novel").
# A section ("novel-documents", "novel-sheets") has a label, its noun in the singular
# ("-one", with a grammatical .gender: masculine, feminine, or other), a count, the
# "+" tooltip and the File menu item for a new file, a hint for when it is empty, and the
# home screen button that starts the first file.

novel = Novel
novel-description = Chapters, character sheets, and the rules of your world

novel-research = Research
novel-research-hint = PDFs, images, and notes — press + to add.

novel-documents = Chapters
novel-documents-one = chapter
    .gender = other
novel-documents-count = { $count ->
    [one] { $count } chapter
   *[other] { $count } chapters
}
novel-documents-new = New chapter
novel-documents-new-menu = New Chapter
novel-documents-empty = No chapters yet — press + to start one.
novel-documents-first = Write the first chapter

novel-sheets = Characters
novel-sheets-one = character
    .gender = other
novel-sheets-count = { $count ->
    [one] { $count } character
   *[other] { $count } characters
}
novel-sheets-new = New character
novel-sheets-new-menu = New Character
novel-sheets-empty = No characters yet — press + to add one.
novel-sheets-first = Add the first character
# What the assistant panel says it sends.
novel-sheets-sent = { $count ->
    [one] { $count } character sheet
   *[other] { $count } character sheets
}

novel-ask-hint = Ask anything, or guide the buttons above (“more tension, shorter sentences”)

novel-action-rewrite = Rewrite selection
    .hint = Rewrite the selected passage
novel-action-continue = Continue the scene
    .hint = Write what comes next at the cursor
novel-action-alternatives = Three alternatives
    .hint = Three different versions of the selection
novel-action-continuity = Check continuity
    .hint = Review this chapter against the canon

novel-context-world-rules = world rules
