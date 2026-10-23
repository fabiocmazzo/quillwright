# Quillwright 界面文字（简体中文）。
# 原文是 ../en/quillwright.ftl，其中说明了各个变量。请保持相同的 id。
# 中文没有单复数之分，所以大多数消息不需要 $count 选择。

## Counts

words = { $words } 字
thousands = { $count }千

## Menus

menu-file = 文件
menu-view = 视图
menu-assistant = 助手
menu-tools = 工具
menu-help = 帮助
menu-lint = 检查 Markdown
menu-format = 格式化 Markdown
menu-shortcuts = 键盘快捷键
menu-add-research = 添加资料文件...
    .hint = 把 PDF、图片和笔记复制到项目的资料文件夹
menu-new-project = 新建项目...
menu-open-project = 打开项目...
menu-project-home = 项目主页
menu-new-blank = 新建空白文档
menu-open-file = 打开文件...
menu-save = 保存
menu-save-as = 另存为...
menu-autosave = 自动保存
    .hint = 停止输入片刻后自动保存当前文件
menu-history = 版本历史...
    .hint = 此文件的早期版本，保存在这台电脑上
menu-export = 导出书稿...
menu-exit = 退出
menu-assistant-panel = 助手面板
menu-assistant-settings = 助手设置...
menu-theme = 主题
menu-theme-system = 跟随系统
menu-focus = 专注模式
menu-sidebar = 侧边栏
menu-search = 搜索项目...
menu-reference = 参考面板
    .hint = 最近打开的资料文件
menu-typewriter = 打字机滚动
    .hint = 让正在书写的行保持在屏幕中间
menu-dim = 淡化其他段落
    .hint = 除正在书写的段落外，其余内容都变淡
menu-line-numbers = 行号
menu-larger = 放大文字
menu-smaller = 缩小文字
menu-default-size = 默认文字大小
menu-session-goal = 本次目标（字数）
    .hint = 0 表示不显示目标
menu-language = 界面语言
menu-language-system = 跟随系统

## Keyboard shortcuts (Help → Keyboard Shortcuts)

shortcuts-title = 键盘快捷键
shortcut-focus = 专注模式（按 Esc 退出）
shortcut-sidebar = 显示 / 隐藏侧边栏
shortcut-assistant = AI 助手面板
shortcut-export = 导出书稿
shortcut-send = 发送助手请求
shortcut-typewriter = 打字机滚动
shortcut-dim = 淡化其他段落
shortcut-text-size = 放大 / 缩小文字
shortcut-new-document = 新建章节
shortcut-blank = 新建空白文档
shortcut-open = 打开文件
shortcut-save = 保存
shortcut-save-as = 另存为
shortcut-undo = 撤销 / 重做
shortcut-find = 查找 / 替换
shortcut-search = 搜索整个项目
shortcut-delete-line = 删除当前行
shortcut-page = 翻动一页
shortcut-lint = 检查 / 格式化 Markdown
shortcut-help = 显示 / 隐藏快捷键
shortcut-quit = 退出

## Sidebar

sidebar-home = 主页
    .hint = 项目主页
sidebar-open = 打开…
    .hint = 打开其他项目
sidebar-outline = 大纲
sidebar-notes = 笔记
sidebar-add-research = 添加资料文件
sidebar-all-files = 所有文件
sidebar-folder-unavailable = 此文件夹不可用。
create-file-failed = 无法创建文件：{ $error }

## Status bar and window title

status-edited = 已编辑
    .autosave = 停顿时会自动保存
    .unsaved = 有未保存的更改（按 Ctrl+S 保存）
status-saved = 已保存
status-not-saved = 尚未保存
status-goal = 本次目标：{ $words } / { $goal } 字
status-session = 本次 { $words }
    .hint = 本次字数只计算你在这里输入的内容
status-manuscript = 书稿共 { $words }
status-words-in = 本{ $document } { $words } 字
status-reading = 约 { $minutes } 分钟读完
status-selected = 已选 { $words }
status-line = 第 { $line } 行
title-home = 主页
title-untitled = 未命名

## Home screen

home-tagline = 为长篇故事准备的一间静室。
home-current-project = 当前项目
home-continue = 继续写作  “{ $title }”
home-new-project = 新建项目…
home-open-project = 打开项目…
home-blank-page = 空白页
home-recent = 最近的项目
home-keys = F11 专注模式   ·   Ctrl+B 侧边栏   ·   Ctrl+H 所有快捷键

## Focus mode

focus-goal = 本次 { $words } / { $goal }
focus-unsaved = 未保存
focus-hint = 专注模式 — 按 F11 或 Esc 退出

## Buttons used in several places

button-save = 保存
button-discard = 放弃
button-cancel = 取消
button-close = 关闭
button-create = 创建
button-save-and-exit = 保存并退出
button-discard-and-exit = 放弃并退出

## Unsaved changes

discard-title = 未保存的更改
discard-exit = 有未保存的更改。退出前要保存吗？
discard-new-document = 有未保存的更改。新建文档前要保存吗？
discard-home = 有未保存的更改。回到项目主页前要保存吗？
discard-open-file = 有未保存的更改。打开其他文件前要保存吗？
discard-create-project = 有未保存的更改。创建项目 { $path } 前要保存吗？
discard-open-project = 有未保存的更改。打开项目 { $path } 前要保存吗？
discard-open = 有未保存的更改。打开 { $name } 前要保存吗？

## File changed on disk

conflict-title = 磁盘上的文件已更改
conflict-this-file = 此文件
conflict-changed = 你有未保存的编辑时，{ $name } 在 Quillwright 之外被更改了（例如被某个智能体）。
conflict-deleted = { $name } 已在 Quillwright 之外被删除或移动。
conflict-unreadable = 无法再读取 { $name }：{ $error }
conflict-reload = 从磁盘重新载入
    .hint = 放弃你未保存的编辑
conflict-keep = 保留我的版本
    .hint = 继续编辑；保存时会覆盖磁盘上的版本
watch-degraded = 文件监视功能受限：{ $warning }

## Opening and saving

dialog-open-project = 打开项目
dialog-choose-parent = 选择所在文件夹
cannot-read = 无法读取 { $path }：{ $error }
open-failed = 无法打开 { $path }：{ $error }
save-failed = 保存出错：{ $error }
project-create-failed = 无法创建项目 { $path }：{ $error }
project-open-failed = 无法打开项目 { $path }：{ $error }

## New Project

new-project-title = 新建项目
new-project-name = 项目文件夹名称
new-project-parent = 创建位置
new-project-choose = 选择文件夹...
new-project-kind = 项目类型
new-project-language = 模板语言
new-project-creates = 将创建 { $files }。
new-project-note = 模板是可选的。不会覆盖已有的文件夹。

## Find and replace

find-title = 查找 / 替换
find-next = 下一个
find-previous = 上一个
find-replace = 替换
find-replace-all = 全部替换
find-found = 找到匹配
find-none = 未找到匹配
find-no-previous = 没有上一个匹配
find-empty = 请输入要查找的文字
find-select = 请选择一个匹配，或使用查找
find-mismatch = 所选内容与查找文字不符
find-replaced = 已替换所选内容
find-nothing-to-replace = 没有可替换的匹配
find-replaced-all = 已替换 { $count } 处
line-deleted = 已删除该行

## Lint and format tools (Tools menu)

tool-output = 工具输出
tool-output-none = 没有输出。
tool-no-lint = 尚未配置检查命令。请在 config.toml 的 [tools] 中添加 lint 项。
tool-no-format = 尚未配置格式化命令。请在 config.toml 的 [tools] 中添加 format 项。
tool-empty = 配置的工具命令为空。
tool-save-first = 对当前文件运行此工具前请先保存，或关闭 *_use_open_file。
tool-not-a-file = 当前路径“{ $path }”不是文件。
tool-no-file = 当前没有打开的文件。请先保存文档，或关闭 *_use_open_file。
tool-temp-failed = 无法准备临时文件：{ $error }
tool-run-failed = 无法运行“{ $tool }”：{ $error }
project-name-invalid = 请输入文件夹名称，不要包含路径分隔符。
project-name-single = 请只使用一个文件夹名称。
project-parent-missing = 请选择一个已存在的文件夹。
button-copy = 复制

## Assistant panel

assistant-title = 助手
assistant-done = 完成
assistant-settings = 设置
assistant-intro = Quillwright 会运行你电脑上安装的 AI 工具，并在这里显示它的回答。只有在你接受建议时，书稿才会改变。
assistant-choose-tool = 选择工具
assistant-command = 命令
assistant-prompt-argument = 把提示作为最后一个参数传入（默认：stdin）
assistant-command-note = 命令在项目文件夹中运行。登录信息和 API 密钥由工具自己保管。
preset-claude-note = 使用你的 Claude Code 登录。安装：https://claude.com/claude-code
preset-codex-note = 使用你的 Codex CLI 登录，在只读沙盒中运行。
preset-ollama-note = 运行本地模型；书稿不会离开你的电脑。
assistant-open-document = 请打开一个{ $document }来使用助手。
assistant-selected = 已选 { $words } 字
assistant-cursor = 光标在第 { $line } 行
assistant-needs-selection = 请先选择一段文字
assistant-ask = 提问
assistant-sends = 将{ $what }发送给 `{ $tool }`。
assistant-this-document = 本{ $document }
assistant-sent-passages = { $count } 段参考文字
assistant-passages = 参考文字
assistant-clear = 清除
assistant-remove-passage = 不再发送这段文字
assistant-writing = 正在写作…
assistant-details = 详细信息
assistant-prompt-sent = 已发送的提示
assistant-revision = 建议的修改
assistant-continuation = 建议的续写
assistant-answer = 回答
assistant-replace = 替换所选内容
assistant-insert = 插入到光标处
assistant-retry = 重试
assistant-other-file = 这个回答属于另一个文件；请重新打开该文件后再应用。
assistant-empty-answer = 助手返回了空的回答。
assistant-select-first = 请先在编辑器中选择一段文字。
assistant-start-failed = 无法启动 `{ $tool }`：{ $error }。
assistant-not-found = 它是否已安装并在 PATH 中？请检查助手设置。
assistant-passage-changed = 自发出请求以来，所选文字已改变。请改为复制该建议。
job-lost = 进程丢失
job-failed = 命令失败（{ $status }）

## Autosave and version history

autosave-failed = 自动保存失败：{ $error }
history-copy-failed = 无法保存历史副本：{ $error }
history-title = 版本历史 — { $title }
history-read-failed = 无法读取此副本：{ $error }
history-empty = 此文件还没有早期版本。
    .when = 当保存替换文件时（最多每 10 分钟一次，大段删除前总会保存），以及智能体的更改替换编辑器中的文字时，都会保存一份副本。
history-compared = 与编辑器中的文字比较：带删除线的文字只在编辑器中，高亮的文字只在此版本中。
history-restore = 恢复此版本
    .hint = 替换编辑器中的文字。当前文字会先保存到历史中。
history-restored = 已恢复早期版本
history-today = 今天 { $time }
history-yesterday = 昨天 { $time }
history-this-year = %m月%d日 %H:%M
history-older = %Y年%m月%d日 %H:%M

## Spelling

spelling-none = 没有拼写建议
spelling-no-suggestions = 没有建议
spelling-add = 添加到项目词典
    .hint = 把“{ $word }”添加到项目文件夹中的 { $file }
spelling-add-failed = 无法更新 { $file }：{ $error }
spelling-ignore = 本次忽略
spelling-check = 拼写检查
    .hint = 为拼错的单词加下划线；右键单击可查看建议
spelling-language = 拼写语言
spelling-language-book = 本书（保存在 metadata.yaml 中）
spelling-language-default = 未设置语言的书的默认值
spelling-loading = 正在载入 { $language }…
spelling-checking = 正在检查 { $language }
spelling-folder = 打开词典文件夹
    .hint = 在这里放入 Hunspell 的 .aff 和 .dic 文件以支持更多语言：
        { $path }
spelling-no-dictionary = 没有 { $language } 的词典
spelling-load-failed = 无法载入 { $language } 词典
write-failed = 无法写入 { $path }

## Project search (sidebar)

search-hint = 搜索项目
search-searching = 正在搜索…
search-none = 没有结果
search-results = { $files ->
    [one] { $count } 个结果
   *[other] { $count } 个结果，在 { $files } 个文件中
}
search-reading = 首次读取 { $title }…
search-more = 正在搜索…
search-failed = { $count } 个文件无法读取
search-and-more = …还有 { $count } 个
search-note = 笔记
search-page = 第 { $page } 页

## Reference panel (research files)

dialog-add-research = 添加资料文件
reference-unreadable = 无法读取该文件。
reference-open-failed = 无法打开此文件：{ $error }
reference-missing = 此文件已被移动或删除。
reference-add-failed = 无法添加资料文件：{ $error }
reference-read-only = 只读
reference-edit = 编辑
    .hint = 在编辑器中打开此文件
reference-actual-size = 实际大小
reference-fit-width = 适合宽度
reference-system-viewer = 用系统查看器打开
reference-use = 用作上下文
    .hint = 把所选文字及其出处发送给助手
reference-click-actual = 单击查看实际大小
reference-click-fit = 单击以适合面板
reference-line = 第 { $line } 行
reference-lines = 第 { $first }–{ $last } 行
reference-unsupported = Quillwright 无法显示这种文件。
reference-cut = 此文件的其余部分未显示
file-text = 文本
file-notes = 笔记
file-image = 图片
file-pdf = PDF
file-other = 文件

## PDF viewer

pdf-unreadable = 无法读取该 PDF。
pdf-no-pages = 此 PDF 没有页面。
pdf-password = 此 PDF 受密码保护，Quillwright 暂时无法打开。
pdf-invalid = 此文件看起来不是 Quillwright 能读取的 PDF。
pdf-page-failed = 无法显示此页。
pdf-page-number = 第 { $page } 页
pdf-pages = 第 { $first }–{ $last } 页
pdf-no-text = 此 PDF 没有可搜索的文字。
pdf-reading = 已读取 { $percent }%
pdf-no-matches = 没有匹配
pdf-match-of = 第 { $current } 个，共 { $total } 个
pdf-matches = { $count } 个匹配
pdf-page = 第
    .hint = 输入页码，或拖动
pdf-page-count = 页，共 { $count } 页
pdf-zoom-in = 放大
pdf-zoom-fit = 使页面适合面板
pdf-zoom-out = 缩小
pdf-find = 在此 PDF 中查找
pdf-previous = 上一个匹配
pdf-next = 下一个匹配

## Export Manuscript

export-title = 导出书稿
export-book-title = 标题
export-author = 作者
export-language = 语言
    .hint = 用于断字，也供电子阅读器使用。留空：使用 metadata.yaml 中的 lang，否则为英语。
export-page-size = 页面大小
export-toc = 目录
export-summary = { $documents } · { $words }，按侧边栏顺序
export-uses = 使用 { $file }
export-styles = 样式来自 { $file }
export-running = 正在导出…
export-start = 导出…
export-nothing = 还没有可导出的内容。
export-done = 已导出 { $file }
export-open = 打开
export-show = 在文件夹中显示
export-save-failed = 无法保存 { $path }：{ $error }
export-stopped = 导出意外停止。
export-metadata-ignored = 已忽略 { $file }：{ $error }
export-write-failed = 无法写入文件：{ $error }
export-no-styles = { $file } 没有样式，已忽略。
export-image-skipped = 未包含图片：{ $file }
format-docx = Word (.docx)
    .hint = 适合编辑、智能体和 Google 文档
format-epub = EPUB 电子书 (.epub)
    .hint = 适合电子阅读器和试读读者
format-pdf = PDF (.pdf)
    .hint = 适合打印和以固定版式分享
format-odt = OpenDocument (.odt)
    .hint = 适合 LibreOffice
format-html = 网页 (.html)
    .hint = 一个独立完整的页面
page-a5 = A5（148 × 210 毫米）
page-a4 = A4（210 × 297 毫米）
page-letter = 美国信纸（8.5 × 11 英寸）
page-trade = 平装书（6 × 9 英寸）

## Themes (View → Theme)

theme-paper = 纸张
theme-sepia = 复古
theme-night = 夜间
reference-loading = 正在载入…
