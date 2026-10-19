# The code review screen

The screen this repository's console plugin bundle carries (`client/`). What is read from which version, how
paths are fenced and what is excluded, and who writes reviews, threads and items are `docs/code-review.md`; this
document is the screen's shape. Every word it shows stands in the bundle's own Korean pack, by key.

## Where it stands

- **The place the owner browses, reads and reviews** the checkout each worker works in. The host console serves
  it at `/code`, its navigation entry named by the pack beside the code graph's. It opens only behind the
  console's sign-in. Code is only read; what is written is reviews, words and items.
- **What it is built with.** The editor is the read-only Monaco the code graph and the document viewer use, and
  the sections are the same splitter and tabs as those two screens. VS Code's workbench is not brought in — it
  swaps the whole editor bundle for its own and can stand only once per page, which would put the other screens
  using the same editor under it too, and its explorer and search read the file system while this screen's tree
  and search read from a commit on the service. No docking library is brought in either — the console's sections
  are one set of splitters and narrow-screen tabs, and a second layout system would move separately inside the
  same console. Switching several files by tab and a review popping up beside a line are built with the editor's
  markers and the space between lines.

## Sections

- **Three sections.** On a wide screen the left is the explorer, the middle the code, the right the review. The
  explorer and review sections are folded by the buttons at the two ends of the code's toolbar, and the code
  section has an irreducible share. The dragged width is remembered by that browser. On a narrow screen the code
  stands alone: the explorer opens from the toolbar's first button and the review section from the toolbar's
  menu, each as a sheet rising from the bottom, which back closes. Choosing a file in the explorer sheet puts it
  away and opens the file, and pressing a review mark in the code opens that thread in the review sheet; going to
  a review's place from the sheet puts it away and brings the code there.
- The tab and section borders split at the same width as the document viewer. The address carries the worker,
  commit, path, line, chosen folder and open review, so the place can be handed over or reopened.
- **The explorer and the review section keep their scrolls at their edges.** The fixed parts — the worker choice,
  the search, a thread's head, the words box — take the section's padding, and the tree, the found lines, the
  review list and the thread reach the section's edges, scroll on their own and line their rows up with the fixed
  parts' text.

## The explorer

- At the top a worker is chosen. A worker stands by display name, and a worker with an unreadable checkout carries
  a different mark and writes the reason beside it. Choosing such a worker stands only the reason in the explorer.
- Below is a search box, beside which a dropdown chooses search by name or by content. Searching by name unfolds
  only the matched files and their ancestors and marks the matched letters. Searching by content stands a list of
  matched lines grouped per file instead of the tree, and pressing a line opens that file at that line. If the
  results hit the ceiling it says so.
- The tree stands folders first, then in name order, folding per folder. Files and folders wear the Material Icon
  Theme's marks — by the file's whole name, its ending, or the folder's name, open or shut — and submodules and
  links carry a mark of their own and do not open. A file and folder with a review at that commit gets a review
  count. The top line is the whole repository, and a folder review there is a review of the whole repository.
- Choosing a folder shows the review section that folder's reviews, and a bar below the tree stands the chosen
  folder's name and a folder review button. A tree line is itself a pressable place so no more buttons can go in
  it, so work on a folder is handled by this bar outside the line.

## The code

- Open files stand as **file tabs** on the tab bar above the section, pressed to switch, closed and dragged. Tabs
  stand in the order opened, with a count ceiling past which the longest-unseen tab closes. Each tab carries its
  file-type mark and remembers its own commit, and its reading place per commit — a place left in a file at one
  commit is never restored onto another commit's version of it. The open tabs of each worker are kept through the
  host console's tab-state store.
- A markdown file's rendering/code switch stands at the tab bar's right end. The toolbar under the tab bar carries
  the explorer's button, the commit, and at its right end the file's actions — go to definition, show line
  reviews, file review, read again and the properties, folded into one vertical-dots menu — then the review
  section's button.
- **The commit.** The toolbar's commit cell stands the short hash and, on a wide screen, the commit title, and a
  current mark if HEAD. If the work tree has uncommitted changes, their count stands in the properties. Pressing
  the cell opens commit choosing, as wide as its longest entry — commits that touched that file, newest first,
  with short hash, title and time, filterable by title and hash. The current commit and the one being viewed stand
  at the top even if they did not touch that file — a list without those two offers no place to return to.
  Choosing a commit opens the file at that point and the explorer tree becomes that commit's. While viewing a
  non-HEAD commit, below the toolbar stand a band saying a past commit is being viewed and a button back to the
  current one. If the open file is not in that commit, it says so instead of the editor.
- The file stands in a read-only editor, its colour, language, font and line numbers as in the code graph's code
  section. A markdown file opens as code too — reviews attach to lines — and the rendering/code switch lets it be
  read as rendering, where line-review marks and inter-line spaces do not stand. A cut file, a binary file and an
  unread file say so. The editor and renderer come down in their own chunk when this section first opens.

## Reviewing

- **Reviewing a line.** Hovering the gutter beside a line number pops a plus mark on that line, and pressing it
  writes that line's review. Dragging to select several lines pops a review button beside the selection, and
  pressing writes that line range's review. With lines chosen, holding shift and pressing another line number
  widens the range to that line. The writing box opens right below the chosen lines, among the code — where it
  anchors, the words box, send, cancel. Sending attaches the review and that word becomes the thread's first,
  opening that thread in the review section.
- On a narrow screen pressing a line number stands that line on a bar below the code, and pressing another line
  number with one chosen widens the range without shift — a finger has no shift. The bar's review button opens the
  writing box as a centred modal, not among the code. File and folder reviews are written in the same modal on a
  wide screen too.
- **Going to a definition.** Pressing the right button anywhere in the code opens the code's own menu, which
  stands go to definition for the word under the press beside the review button for the lines chosen. F12 does
  the same for the word the cursor stands on, and go to definition stands among the file's actions as well, which
  is a finger's route since a phone has neither a right button nor an F12; there it stands dimmed while the cursor
  is on no word, or on one the search will not take. The definition is looked for in the commit being shown
  (`docs/code-review.md`, 「Search」). One found in the open file scrolls the code to that line and marks it; one
  in another file opens that file as a tab at the same commit, at that line. Several stand as a list beside the
  press, at every width — each row the path, the line number, that line as code and what it declares, and
  pressing one goes there. Where none is found nothing at all happens: no notice and no mark. A jump is a step of
  the back history, and back brings the reader to the file and the line they jumped from.
- **A review beside a line.** A line review at the open commit carries a speech-bubble mark in the line-number
  gutter, and those lines' backgrounds are lightly coloured. Below the last line the review points at, among the
  code, it stands as one folded cell — the first word's head, the count of words, an answering mark if the agent
  is answering, the item number if one was filed. Pressing opens that thread in the review section. The show line
  reviews toggle among the file's actions can hide all the inter-line cells and leave only the gutter marks, and
  that choice is remembered by the browser. On a narrow screen the cell shrinks to one line.

## The review section

- Its head names the reviews alone — the open file is already named by its tab — with a file review or folder
  review button at its right end. Under it stands the review list of the open file (or the chosen folder).
  Reviews at the open commit come first in line order, and reviews at another commit stand below grouped per
  commit under a short hash head. Each line stands the place (line, range, file, folder), the first word's head,
  the count of words and the last word's time. Pressing a review at another commit moves the screen to that commit
  and opens that place.
- Opening a review stands the **thread** instead of the list. The head stands a back button, the place, path and
  line, and the commit's short hash, and pressing goes the code to that place. A review whose commit can no longer
  be read says so and shows the kept excerpt as code. Below it the words stand in order, and the agent's answers
  are drawn as the console's markdown. Sending in the words box stands the word at once; while the agent answers
  it says answering, and when an answer comes the screen takes the event and reads again. A word unanswered because
  of the rest state is marked so. While answering the words box is locked. An item filed from this review stands
  as a number, and pressing opens that item in the host console's TODO panel.

## Filing a TODO

- The thread head's make-a-TODO-item button opens a dialog with a make button. It stands the title, what to do,
  and an optional conclusion, and the to-do box is prefilled with the agent's last answer. Below it the worker to
  assign, the place and commit, and the count of words to load as the original conversation stand as unchangeable
  facts. The item is filed awaiting approval.
- Filing closes the box; the filed item number stands as a notice and stands in the thread too. If the agent is
  answering the button is locked and a note beside it says the item can be made once the answer arrives. If the
  service refuses, the reason is written in the box by its code and the written words are left. Back closes the
  box.

## Loading

- While reading the explorer and files, while waiting on the commit list and search, and while reading a thread,
  the console's loading display is followed. If it cannot be read the reason and a read-again button stand.

## Live updates

- The screen listens to this service's event stream (`docs/api.md`, 「Events」): a `code-review` event reads the
  review list again, and a `code-review-talk` event reads the open thread again.
