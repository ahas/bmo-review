-- The reviews, their threads and the items filed from them (docs/code-review.md, 「The review table」).

-- A review: words left on a place of a commit of a worker's checkout, never deleted. The root is the checkout's
-- path when it was left; the excerpt is the reviewed lines of a line or range review, at that commit.
CREATE TABLE code_reviews (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  worker TEXT NOT NULL,
  root TEXT NOT NULL,
  commit_sha TEXT NOT NULL,
  scope TEXT NOT NULL,
  path TEXT NOT NULL,
  start_line INTEGER,
  end_line INTEGER,
  excerpt TEXT,
  created_at INTEGER NOT NULL
);

CREATE INDEX code_reviews_place ON code_reviews (worker, path, id);

-- One line of a review's thread: who said it, owner or agent, the words and when.
CREATE TABLE code_review_talk (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  review_id INTEGER NOT NULL REFERENCES code_reviews (id),
  speaker TEXT NOT NULL,
  text TEXT NOT NULL,
  said_at INTEGER NOT NULL
);

CREATE INDEX code_review_talk_review ON code_review_talk (review_id, id);

-- An item filed from a review, by the number bmo-job answered for it; nothing here reaches into bmo-job's
-- tables.
CREATE TABLE code_review_todos (
  review_id INTEGER NOT NULL REFERENCES code_reviews (id),
  filed_item INTEGER NOT NULL,
  PRIMARY KEY (review_id, filed_item)
);
