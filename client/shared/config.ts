// What the code review holds itself to (`docs/code-review.md`). How much of a file the viewer
// carries before it says the rest was cut, how far into a file it looks for a NUL before calling it
// binary, how many entries of one commit's tree come over at once, how many commits the picker
// offers, what one content search may return and how long it may take, how much of a matched line
// is shown, how many lines a line review may span and how much of them it keeps, how many files
// stand open as tabs, and the width at and below which the three parts become tabs.
export const CODE_VIEW = {
  fileCap: 1_000_000,
  binaryProbe: 8_000,
  treeCap: 50_000,
  commitCap: 100,
  searchQueryMax: 200,
  searchPerFile: 5,
  searchTotal: 200,
  searchTimeoutMs: 10_000,
  // A definition search is one expression standing for every language at once, which costs about
  // ten times a plain search and still answers the largest checkout in under a second. It is a
  // jump the reader is waiting on, so its ceiling is seconds rather than the content search's ten.
  //
  // Two numbers stand for one file because the search deliberately over-matches: an expression
  // holding every language's shapes finds, in a file of one of them, lines another language would
  // have declared something on — a bare call at the head of a line is a shell function's own shape
  // — and those are dropped by the file's own language afterwards. So the lines read from a file
  // are the larger number and the definitions kept out of them the smaller one, or five lines that
  // declare nothing would stand in for the five declarations under them.
  definitionScanPerFile: 40,
  definitionPerFile: 5,
  definitionTotal: 50,
  definitionTimeoutMs: 5_000,
  matchLineMax: 300,
  reviewExcerptMax: 20_000,
  openTabsMax: 10,
  tabsMaxPx: 1023
} as const;

// How long a code review's title, its "what to do" and its conclusion may be when a row is filed
// out of it.
export const CODE_REVIEW_TITLE_LIMIT = 200;
export const CODE_REVIEW_TODO_LIMIT = 8_000;
export const CODE_REVIEW_CONCLUSION_LIMIT = 4_000;

// How long one turn of a review's talk may be: a thing said in a narrow panel, not a document.
export const TALK_LIMIT = 2_000;
