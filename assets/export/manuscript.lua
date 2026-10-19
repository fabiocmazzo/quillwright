-- Quillwright export filter for pandoc.
-- Starts every chapter (level-1 heading) on a new page and turns scene
-- breaks (***, ---) into a centered "* * *" instead of a horizontal line.

local function scene_break()
  if FORMAT:match('docx') then
    -- Centered directly, so it looks right without a reference.docx.
    return pandoc.RawBlock('openxml',
      '<w:p><w:pPr><w:pStyle w:val="SceneBreak"/><w:jc w:val="center"/></w:pPr>'
      .. '<w:r><w:t xml:space="preserve">*   *   *</w:t></w:r></w:p>')
  elseif FORMAT:match('odt') then
    return pandoc.Div(pandoc.Para({ pandoc.Str('*   *   *') }), { ['custom-style'] = 'Scene Break' })
  elseif FORMAT:match('typst') then
    return pandoc.RawBlock('typst', '#align(center)[\\* #h(1em) \\* #h(1em) \\*]')
  elseif FORMAT:match('latex') then
    return pandoc.RawBlock('latex', '\\begin{center}*\\quad*\\quad*\\end{center}')
  elseif FORMAT:match('html') or FORMAT:match('epub') then
    return pandoc.RawBlock('html', '<p class="scene-break" style="text-align: center; text-indent: 0">*&#160;&#160;&#160;*&#160;&#160;&#160;*</p>')
  end
  return nil
end

local function page_break()
  if FORMAT:match('docx') then
    return pandoc.RawBlock('openxml', '<w:p><w:r><w:br w:type="page"/></w:r></w:p>')
  elseif FORMAT:match('odt') then
    return pandoc.RawBlock('opendocument', '<text:p text:style-name="Pagebreak"/>')
  elseif FORMAT:match('typst') then
    return pandoc.RawBlock('typst', '#pagebreak(weak: true)')
  elseif FORMAT:match('latex') then
    return pandoc.RawBlock('latex', '\\clearpage')
  elseif FORMAT:match('html') and not FORMAT:match('epub') then
    -- Only affects printing; EPUB already splits chapters into files.
    return pandoc.RawBlock('html', '<div style="break-before: page"></div>')
  end
  return nil
end

function Pandoc(doc)
  local has_title = doc.meta.title ~= nil
  local blocks = {}
  for _, block in ipairs(doc.blocks) do
    if block.t == 'HorizontalRule' then
      table.insert(blocks, scene_break() or block)
    else
      -- No break before the first chapter unless a title page comes before it.
      if block.t == 'Header' and block.level == 1 and (has_title or #blocks > 0) then
        local br = page_break()
        if br then table.insert(blocks, br) end
      end
      table.insert(blocks, block)
    end
  end
  doc.blocks = blocks
  return doc
end
