# The Legal case kind as the app shows it. See ../../novel/locales/en.ftl for what each
# message is.

legal = Legal case
legal-description = Filings for one case, with its facts, timeline, parties, and evidence

legal-research = Case Files
legal-research-hint = Court records, exhibits, and other PDFs — press + to add.

legal-documents = Filings
legal-documents-one = filing
    .gender = other
legal-documents-count = { $count ->
    [one] { $count } filing
   *[other] { $count } filings
}
legal-documents-new = New filing
legal-documents-new-menu = New Filing
legal-documents-empty = No filings yet — press + to start one.
legal-documents-first = Write the first filing

legal-sheets = Parties
legal-sheets-one = party
    .gender = other
legal-sheets-count = { $count ->
    [one] { $count } party
   *[other] { $count } parties
}
legal-sheets-new = New party
legal-sheets-new-menu = New Party
legal-sheets-empty = No parties yet — press + to add one.
legal-sheets-first = Add the first party
legal-sheets-sent = { $count ->
    [one] { $count } party sheet
   *[other] { $count } party sheets
}

legal-ask-hint = Ask anything, or guide the buttons above (“more formal”, “cite the exhibits”)

legal-action-rewrite = Rewrite selection
    .hint = Rewrite the selected passage
legal-action-draft = Draft from facts
    .hint = Draft the text at the cursor from the facts and evidence
legal-action-check = Check against facts
    .hint = Compare this filing with the facts, timeline, and evidence
legal-action-questions = Questions for the client
    .hint = What is still unknown, as questions for the client

legal-context-facts = facts
legal-context-timeline = timeline
legal-context-evidence = evidence list
legal-context-client-questions = client questions
