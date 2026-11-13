# The code review screen

The screen this repository's console bundle carries (`client/`), attached to a console by the manifest the
service answers (`docs/api.md`, 「The console face」). What is read from which revision, how paths are fenced and what is
excluded, and how reviews, comments and filings behave are `docs/code-review.md`; this document is the screen's
shape. Every word it shows stands in the bundle's own pack, by key under `code.`.

## Where it stands

- **The place a reader browses, reads and reviews** each project's checkout. A console serves it at `/code`,
  under the title and icon the manifest names, to a reader holding the `member` role or the `administrator`. Code is only read; what is written is reviews, comments and
  filed items.
- **What it is built with.** The editor is bmo-ui's read-only Monaco code view, and the sections are bmo-ui's
  splitter and tabs. No docking library and no workbench are brought in: the console's sections are one set of
  splitters and narrow-screen tabs, and a second layout system would move separately inside the same console.
  Switching several files by tab and a comment standing beside a line are built with the editor's markers and the
  space between lines.
- **Under the console's title bar.** The screen's route asks for the console's bar in its meta, so a console that
  offers its bar (bmo-ui's `docs/console-bundles.md`, 「How a bundle mounts」, the title bar) draws it over the screen
  under the manifest's title and icon, and the screen draws no bar of its own. Found without that offer it draws its
  own bar, named and marked the same. Either bar holds the screen's name and mark alone: the screen lays nothing
  beside the title, neither an action nor a way back. Its own row under the bar is the code's tab bar and toolbar,
  which hold what acts on the open file (「The code」).
- **Every call goes to the review service** at the address bmo-ui's `relay` module answers for `review`, under the
  service's screen prefix `/screen`, the event stream included (`docs/api.md`, 「The console face」).

## Sections

- **Three sections.** On a wide screen the left is the explorer, the middle the code, the right the review. The
  explorer and review sections are folded by the buttons at the two ends of the code's toolbar, and the code
  section has an irreducible share. The dragged width is remembered by that browser. On a narrow screen the code
  stands alone: the explorer opens from the toolbar's first button and the review section from the toolbar's
  menu, each as a sheet rising from the bottom, which back closes. Choosing a file in the explorer sheet puts it
  away and opens the file, and pressing a comment mark in the code opens that review in the review sheet; going to
  a comment's place from the sheet puts it away and brings the code there. On a phone the new review button floats
  over the code (「The review section」), and the code section ends the button's room above the screen's foot, the
  layer's create room, so its last line and the chosen lines' bar stand clear of it.
- The address carries the project, revision, path, line, chosen folder and open review, so the place can be
  handed over or reopened.
- **The explorer and the review section keep their scrolls at their edges.** The fixed parts (the project choice,
  the search, a review's head, the words box) take the section's padding, and the tree, the found lines, the
  review list and the comments reach the section's edges, scroll on their own and line their rows up with the
  fixed parts' text.

## The explorer

- At the top a project is chosen, by name. A project with an unreadable checkout carries a different mark and
  writes the reason beside it. Choosing such a project stands only the reason in the explorer.
- Below is a search box, beside which a dropdown chooses search by name or by content. Searching by name unfolds
  only the matched files and their ancestors and marks the matched letters. Searching by content stands a list of
  matched lines grouped per file instead of the tree, and pressing a line opens that file at that line. Where the
  service says there is more, the list says the results were cut.
- The tree stands folders first, then in name order, folding per folder. Files and folders wear the file icon
  marks bmo-ui gives (by the file's whole name, its ending, or the folder's name, open or shut), and submodules
  and links carry a mark of their own and do not open. A file or folder with comments at that revision gets a
  comment count. The top line is the whole repository, and a folder comment there is a comment on the whole
  repository.
- Choosing a folder shows the review section that folder's comments, and a bar below the tree stands the chosen
  folder's name and, at its right end, the folder comment button, an icon whose tooltip names it. A tree line is
  itself a pressable place so no more buttons can go in it, so work on a folder is handled by this bar outside the
  line.

## The code

- Open files stand as **file tabs** on the tab bar above the section, pressed to switch, closed and dragged. Tabs
  stand in the order opened, with a count ceiling past which the longest-unseen tab closes. Each tab carries its
  file-type mark and remembers its own revision, and its reading place per revision: a place left in a file at one
  revision is never restored onto another revision's version of it. The open tabs of each project are kept in
  that browser.
- The toolbar under the tab bar carries the explorer's button, the revision, and at its right end the file's
  actions (go to definition, show line comments, file comment, read again and the properties, folded into one
  vertical-dots menu), then the review section's button. Each of its buttons is the component library's small
  button at its own size, centred in the row, its hover and press filling only its own shape; how much room a cell
  keeps is bmo-ui's toolbar's to decide off what the cell holds, and the screen sets none of it.
- **The revision.** The toolbar's revision cell stands the short hash and, on a wide screen, the commit's subject,
  and a current mark if HEAD. If the work tree has uncommitted changes, their count stands in the properties.
  Pressing the cell opens revision choosing, as wide as its longest entry: commits that touched that file, newest
  first, with short hash, subject and time, filterable by subject and hash. The current revision and the one
  being viewed stand at the top even if they did not touch that file, since a list without those two offers no
  place to return to. Choosing a revision opens the file at that point and the explorer tree becomes that
  revision's. While viewing a revision other than HEAD, below the toolbar stand a band saying a past revision is
  being viewed and a button back to the current one. If the open file is not in that revision, it says so instead
  of the editor.
- The file stands in a read-only editor, its colour, language, font and line numbers as bmo-ui's code view gives
  them. A markdown file opens as code too, so comments attach to its lines. A cut file, a binary file and an
  unread file say so. The editor comes down in its own chunk when this section first opens.

## Reviewing

- **The review being written.** Comments are written into a review of the revision being viewed: the one open in
  the review section where it is of that revision, else the reader chooses one of that revision's reviews or opens
  a new one in the writing box, by a title. A new review is opened with the first comment written into it at
  once.
- **The writer's name.** Every review and comment is signed with the name the reader gives in the writing box. The
  box asks for it the first time and that browser keeps it, prefilling it each time after.
- **Commenting on a line.** Hovering the gutter beside a line number pops a plus mark on that line, and pressing it
  writes that line's comment. Dragging to select several lines pops a comment button, its word alone, beside the
  selection, and pressing writes that line range's comment. With lines chosen, holding shift and pressing another
  line number widens the range to that line. The writing box opens right below the chosen lines, among the code:
  where it anchors, the review it goes into, the name, the words box, send, cancel. Sending writes the comment and
  opens its review in the review section.
- On a narrow screen pressing a line number stands that line on a bar below the code, and pressing another line
  number with one chosen widens the range without shift, since a finger has no shift. The bar's cancel and comment
  buttons are words alone, and the comment button opens the writing box as a centred modal, not among the code.
  File, folder and whole-review comments are written in the same modal on a wide screen too.
- **Going to a definition.** Pressing the right button anywhere in the code opens the code's own menu, which stands
  go to definition for the word under the press beside the comment button for the lines chosen. F12 does the same
  for the word the cursor stands on, and go to definition stands among the file's actions as well, which is a
  finger's route since a phone has neither a right button nor an F12; there it stands dimmed while the cursor is
  on no word. The definition is looked for in the revision being shown (`docs/code-review.md`, 「Search」). One
  found in the open file scrolls the code to that line and marks it; one in another file opens that file as a tab
  at the same revision, at that line. Several stand as a list beside the press, at every width: each row the
  path, the line number, that line as code and what it declares, and pressing one goes there. Where none is found,
  or the word is no name the search takes, nothing at all happens: no notice and no mark. A jump is a step of the
  back history, and back brings the reader to the file and the line they jumped from.
- **A comment beside a line.** A line comment at the open revision carries a speech-bubble mark in the line-number
  gutter, and those lines' backgrounds are lightly coloured. Below the last line it points at, among the code, it
  stands as one folded cell: its review's title, the writer's name and the comment's first words. Pressing opens
  that review in the review section. The show line comments toggle among the file's actions can hide all the
  inter-line cells and leave only the gutter marks, and that choice is remembered by the browser. On a narrow
  screen the cell shrinks to one line.

## The review section

- **No button beside a title.** A head here is a title on a row of its own and, under it, a row holding the way back
  at its left end, what states the thing in the middle, and the actions at its right end. Every button in those rows
  is the component library's small button, its hover filling only its own shape.
- **A new review is the screen's create button,** bmo-ui's one create button (its `docs/components.md`,
  「Creating」), named a new review and answering the key `n`; pressing it opens the writing box for a new review.
  The screen stands no other button of its own that creates: the folder comment button and the line comment buttons
  act on a chosen place and carry an icon or a word, never both. Where the screen is wider than a phone it is the grey
  icon at the right end of the row under the reviews' head, there while the review list stands; its key acts while
  the button stands and nothing floats over the screen, so not while the review sheet is open on a narrow screen. On
  a phone it floats at the screen's bottom right over the code, outside the sheets, which cover it while they stand;
  nothing stands for it in the review sheet.
- Its head names the reviews on a row of their own, and the row under it stands the new review button at its right
  end. On a narrow screen the sheet's own title names the reviews, so only that row stands, and on a phone, where the
  button floats, the row stands nothing. Under it stands the list of the project's reviews: those of the open
  revision first, newest first, then those of other revisions
  grouped per revision under a short hash head. Each row stands the title, the state, the writer's name, the count
  of comments and when it last changed. Pressing a review of another revision moves the screen to that revision and
  opens the review there.
- Opening a review stands the **review** instead of the list. Its title stands on a row of its own. The row under
  it stands the back button to the list at its left end, then the state, the writer's name and the revision's short
  hash, which is a button going to that revision, and at its right end the close or reopen button and the file an
  item button. The items filed from the review stand as their numbers on a line under that row. Below it the
  comments stand in order, each with its writer's name, its time, its place (the whole, a folder, a file, a line or
  a range) and its words as plain text, line breaks kept. Pressing a comment's place goes the code there. A line
  comment of a review of another revision than the one viewed stands its kept excerpt as code, so it reads even
  where that revision no longer can. At the bottom the words box writes a comment on the whole review, with the
  writer's name.
- Sending stands the comment at once, and the screen reads the review again when the service's event says it
  moved.

## Filing an item

- The review head's file an item button opens a dialog with a file button. It stands the title, the brief and the
  queue the item goes into, the brief prefilled with the review's comments in order, each under its writer's name
  and place, and the queue prefilled with the one that browser filed into last. Below it the review's title and
  revision stand as unchangeable facts.
- Filing closes the box; the filed item's number stands as a notice and stands in the review too. If the service
  refuses, the reason is written in the box by its code and the written words are left. Back closes the box.

## Loading

- While reading the explorer and files, while waiting on the revision list and search, and while reading a
  review, bmo-ui's loading display is followed. If it cannot be read the reason and a read-again button stand.

## Live updates

- The screen reads the service's event stream (`docs/api.md`, 「Events」): a `review.created` or `review.changed`
  event reads the review list again, and a `comment.created` or `review.changed` event of the open review reads
  its comments again. A stream answered `gone` reads every list again.
