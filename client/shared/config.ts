// What the code review screen holds itself to (`docs/screen.md`): how many entries of a revision's tree it reads
// at most, a page at a time, how long the content search's words may be, how many files stand open as tabs,
// and the width at and below which the three parts become one with two sheets. The ceilings of what is written
// are the service's own settings at their defaults (`docs/api.md`, 「Settings」), so a box stops where the
// service would refuse.
export const CODE_VIEW = {
  treeCap: 50_000,
  treePage: 50_000,
  searchQueryMax: 200,
  openTabsMax: 10,
  tabsMaxPx: 1023
} as const;

// A review's title, a comment's words and a writer's name.
export const TITLE_LIMIT = 200;
export const COMMENT_LIMIT = 20_000;
export const AUTHOR_LIMIT = 200;

// The words of an item filed from a review, which the job service judges in the end.
export const BRIEF_LIMIT = 20_000;
