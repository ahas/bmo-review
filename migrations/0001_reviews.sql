-- The reviews, their comments, the items filed from them and the service's events (docs/code-review.md,
-- 「Reviews」; docs/api.md, 「Events」).

-- A review: a titled conversation about one revision of one project, never deleted. Its state is open,
-- answered or closed.
CREATE TABLE reviews (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  project_name TEXT NOT NULL,
  revision TEXT NOT NULL,
  title TEXT NOT NULL,
  state TEXT NOT NULL,
  author_name TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  changed_at INTEGER NOT NULL
);

CREATE INDEX reviews_project ON reviews (project_name, revision, id);

-- A comment on a review: on the whole where it has no path, on a file or folder where it has no line, on a
-- line or a range of a file otherwise, with the lines it points at copied from the review's revision.
CREATE TABLE comments (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  review_id INTEGER NOT NULL REFERENCES reviews (id),
  path TEXT,
  line INTEGER,
  end_line INTEGER,
  excerpt TEXT,
  text TEXT NOT NULL,
  author_name TEXT NOT NULL,
  created_at INTEGER NOT NULL
);

CREATE INDEX comments_review ON comments (review_id, id);

-- An item filed from a review, by the number the job service answered for it.
CREATE TABLE filings (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  review_id INTEGER NOT NULL REFERENCES reviews (id),
  item_id INTEGER NOT NULL,
  created_at INTEGER NOT NULL
);

CREATE INDEX filings_review ON filings (review_id, id);

-- The window of the service's latest events, numbered as they were published, each tied to the review it
-- belongs to.
CREATE TABLE events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  kind TEXT NOT NULL,
  review_id INTEGER NOT NULL,
  at INTEGER NOT NULL,
  data TEXT NOT NULL
);
