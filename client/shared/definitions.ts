// Where a name is declared, read off the shape of the line that declares it.
//
// The console holds no language server and no parser. The checkouts it reads are written in
// TypeScript, Vue, Python, Rust, C#, shell and a tail of others, and a reader stands on whatever
// commit they chose, so what answers "where is this declared" has to work on the text of one
// commit's files and on every one of those languages at once. It is a search: the lines that
// declare a name have a shape, one set of shapes per language family, and git searches the commit
// for them (`docs/code-review.md`, 「Search」).
//
// One table below states what a declaration looks like, and two things are rendered from it — the
// single expression git is given, and this module's own reading of a line that came back, which
// says what kind of declaration it is and drops a line whose language does not own the shape that
// found it. A second table would be a second answer to the same question.
//
// Both renderings are the same text, so it must be a regular expression in two dialects at once:
// the POSIX extended one git grep runs, and JavaScript's own. Nothing here uses what only one of
// them has — no \b, \w, \s or \t escape, no lookaround, no lazy quantifier. A word edge is a
// bracket of its own, a blank is a bracket holding a space and a tab, and the dollar, the one
// character an identifier may carry that a regular expression reads as something else, goes
// through a bracket rather than a backslash.

// The language families a definition is looked for in. A file of any other kind declares nothing
// here: it is read and reviewed like everything else, but a name is not looked for in it.
export type DefinitionLanguage = "ts" | "python" | "rust" | "csharp" | "shell" | "lua";

// What the line declares. The screen names these in its own words; here they are only what the
// shape that matched was looking for.
export type DefinitionKind =
  | "function"
  | "method"
  | "class"
  | "interface"
  | "type"
  | "enum"
  | "const"
  | "struct"
  | "trait"
  | "module"
  | "macro"
  | "re-export";

// A character that cannot be part of a name, which is how a word ends where neither dialect has a
// word boundary of its own. The blank holds a space and a real tab.
const EDGE = "[^A-Za-z0-9_$]";
const OPEN = `(^|${EDGE})`;
const CLOSE = `(${EDGE}|$)`;
const BLANK = "[ \t]";
const INDENT = `^${BLANK}*`;

// What may stand between the head of a line and a TypeScript declaration's own keyword. The
// declaration is anchored to the line rather than taken wherever the keyword appears, which is
// what keeps the `const` of `for (const item of list)` and the `type` of `import { type Row }`
// from reading as declarations of what follows them.
const TS_HEAD = `${INDENT}(export${BLANK}+)?(default${BLANK}+)?(declare${BLANK}+)?(abstract${BLANK}+)?(async${BLANK}+)?`;

interface DefinitionForm {
  language: DefinitionLanguage;
  kind: DefinitionKind;
  // What must stand before the name on the line, and what must follow it.
  before: string;
  after: string;
}

// Every shape a declaration takes, the more particular before the looser within one language,
// since the first that fits a line names what that line declares.
const FORMS: readonly DefinitionForm[] = [
  // TypeScript and JavaScript, and the script a Vue file carries.
  { language: "ts", kind: "function", before: `${TS_HEAD}function(${BLANK}+[*]?${BLANK}*|${BLANK}*[*]${BLANK}*)`, after: `${BLANK}*[(<]` },
  { language: "ts", kind: "class", before: `${TS_HEAD}class${BLANK}+`, after: CLOSE },
  { language: "ts", kind: "interface", before: `${TS_HEAD}interface${BLANK}+`, after: CLOSE },
  { language: "ts", kind: "enum", before: `${TS_HEAD}(const${BLANK}+)?enum${BLANK}+`, after: CLOSE },
  { language: "ts", kind: "type", before: `${TS_HEAD}type${BLANK}+`, after: `${BLANK}*[=<]` },
  { language: "ts", kind: "const", before: `${TS_HEAD}(const|let|var)${BLANK}+`, after: `${BLANK}*[:=]` },
  // A name the file only passes on. It is a place the name is written, so it stands, but it stands
  // after every real declaration.
  { language: "ts", kind: "re-export", before: `${INDENT}export${BLANK}*(type${BLANK}+)?[{]([^}]*${EDGE})?`, after: CLOSE },

  // Python. A method is written exactly as a function is, so it is one shape; a module's own
  // constant is an assignment at the very head of a line, which is what separates it from the
  // bindings inside a body.
  { language: "python", kind: "function", before: `${INDENT}(async${BLANK}+)?def${BLANK}+`, after: `${BLANK}*[(]` },
  { language: "python", kind: "class", before: `${INDENT}class${BLANK}+`, after: `${BLANK}*[(:]` },
  { language: "python", kind: "const", before: "^", after: `${BLANK}*[:=]` },

  // Rust. Every keyword here is reserved, so a shape is anchored to the keyword rather than to the
  // line, and the modifiers that may crowd in front of it — pub, async, unsafe, extern — need no
  // saying.
  { language: "rust", kind: "function", before: `${OPEN}fn${BLANK}+`, after: `${BLANK}*[(<]` },
  { language: "rust", kind: "struct", before: `${OPEN}struct${BLANK}+`, after: CLOSE },
  { language: "rust", kind: "enum", before: `${OPEN}enum${BLANK}+`, after: CLOSE },
  { language: "rust", kind: "trait", before: `${OPEN}trait${BLANK}+`, after: CLOSE },
  { language: "rust", kind: "type", before: `${OPEN}type${BLANK}+`, after: `${BLANK}*[=<]` },
  { language: "rust", kind: "const", before: `${OPEN}(const|static)${BLANK}+(mut${BLANK}+)?`, after: `${BLANK}*[:]` },
  { language: "rust", kind: "module", before: `${OPEN}mod${BLANK}+`, after: CLOSE },
  { language: "rust", kind: "macro", before: `${OPEN}macro_rules!${BLANK}*`, after: CLOSE },

  // C#. A method carries no keyword of its own, so it is known by the modifiers its line opens
  // with and by the brackets that follow the name; stopping before an equals or a semicolon is
  // what keeps a call written on such a line from reading as the declaration.
  { language: "csharp", kind: "class", before: `${OPEN}class${BLANK}+`, after: CLOSE },
  { language: "csharp", kind: "class", before: `${OPEN}record${BLANK}+`, after: CLOSE },
  { language: "csharp", kind: "struct", before: `${OPEN}struct${BLANK}+`, after: CLOSE },
  { language: "csharp", kind: "interface", before: `${OPEN}interface${BLANK}+`, after: CLOSE },
  { language: "csharp", kind: "enum", before: `${OPEN}enum${BLANK}+`, after: CLOSE },
  {
    language: "csharp",
    kind: "method",
    before: `${INDENT}(public|private|protected|internal|static|async|override|virtual|sealed|abstract|partial|new)[^=;]*${EDGE}`,
    after: `${BLANK}*[(<]`
  },

  // Shell. A function is a name with an empty pair of brackets after it, with or without the word
  // in front; a variable is an assignment at the head of its line.
  { language: "shell", kind: "function", before: `${INDENT}(function${BLANK}+)?`, after: `${BLANK}*[(]${BLANK}*[)]` },
  { language: "shell", kind: "function", before: `${INDENT}function${BLANK}+`, after: `(${BLANK}|[{]|$)` },
  { language: "shell", kind: "const", before: `${INDENT}(export${BLANK}+|declare${BLANK}+|local${BLANK}+|readonly${BLANK}+|typeset${BLANK}+)?`, after: "=" },

  // Lua.
  { language: "lua", kind: "function", before: `${OPEN}(local${BLANK}+)?function${BLANK}+`, after: `${BLANK}*[(]` },
  { language: "lua", kind: "const", before: `${OPEN}local${BLANK}+`, after: `${BLANK}*=` }
];

// The ending of a path against the family it belongs to.
const LANGUAGES: Record<string, DefinitionLanguage> = {
  ts: "ts",
  tsx: "ts",
  mts: "ts",
  cts: "ts",
  js: "ts",
  mjs: "ts",
  cjs: "ts",
  jsx: "ts",
  vue: "ts",
  py: "python",
  pyi: "python",
  rs: "rust",
  cs: "csharp",
  sh: "shell",
  bash: "shell",
  zsh: "shell",
  lua: "lua"
};

// The words each family spends on itself. A reader pressing one of them is pointing at the
// language rather than at a name, and there is nothing to go to.
const KEYWORDS: Record<DefinitionLanguage, readonly string[]> = {
  ts: [
    "as", "async", "await", "break", "case", "catch", "class", "const", "continue", "declare", "default", "delete", "do", "else",
    "enum", "export", "extends", "false", "finally", "for", "from", "function", "if", "implements", "import", "in", "instanceof",
    "interface", "keyof", "let", "new", "null", "of", "return", "satisfies", "static", "super", "switch", "this", "throw", "true",
    "try", "type", "typeof", "undefined", "var", "void", "while", "yield"
  ],
  python: [
    "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del", "elif", "else", "except", "False",
    "finally", "for", "from", "global", "if", "import", "in", "is", "lambda", "None", "nonlocal", "not", "or", "pass", "raise",
    "return", "self", "True", "try", "while", "with", "yield"
  ],
  rust: [
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn", "for", "if",
    "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct",
    "super", "trait", "true", "type", "unsafe", "use", "where", "while"
  ],
  csharp: [
    "abstract", "as", "async", "await", "base", "bool", "break", "case", "catch", "class", "const", "continue", "default", "do",
    "double", "else", "enum", "event", "false", "finally", "float", "for", "foreach", "if", "int", "interface", "internal", "is",
    "lock", "namespace", "new", "null", "object", "override", "partial", "private", "protected", "public", "readonly", "record",
    "ref", "return", "sealed", "static", "string", "struct", "switch", "this", "throw", "true", "try", "typeof", "using", "var",
    "virtual", "void", "while"
  ],
  shell: [
    "case", "declare", "do", "done", "elif", "else", "esac", "export", "fi", "for", "function", "if", "in", "local", "readonly",
    "return", "then", "typeset", "until", "while"
  ],
  lua: [
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "if", "in", "local", "nil", "not", "or", "repeat",
    "return", "then", "true", "until", "while"
  ]
};

const KEYWORD_SETS = new Map<DefinitionLanguage, Set<string>>(
  Object.entries(KEYWORDS).map(([language, words]) => [language as DefinitionLanguage, new Set(words)])
);

const EVERY_KEYWORD = new Set(Object.values(KEYWORDS).flat());

// What a name may look like at all: the shape every one of these languages agrees on, two letters
// at least — one letter finds half the repository and points at nothing — and an upper bound no
// identifier anybody writes comes near, so the expression handed to git stays a known size.
const NAME_SHAPE = /^[A-Za-z_$][A-Za-z0-9_$]*$/;
const NAME_LEAST = 2;
const NAME_MOST = 128;

// The family a path belongs to, or nothing where no family owns it.
export function definitionLanguage(path: string): DefinitionLanguage | null {
  const name = path.slice(path.lastIndexOf("/") + 1).toLowerCase();
  const dot = name.lastIndexOf(".");

  return dot <= 0 ? null : (LANGUAGES[name.slice(dot + 1)] ?? null);
}

// The word a reader pressed, as a name worth looking for, or nothing where it is not one. A word
// in a file of no family is still asked for — the reader may be standing in a document — and only
// the words every family spends on itself are refused there.
export function definitionName(word: string | null | undefined, path: string): string | null {
  const name = (word ?? "").trim();

  if (!NAME_SHAPE.test(name) || name.length < NAME_LEAST || name.length > NAME_MOST) {
    return null;
  }

  const language = definitionLanguage(path);
  const spent = language === null ? EVERY_KEYWORD : (KEYWORD_SETS.get(language) ?? EVERY_KEYWORD);

  return spent.has(name) ? null : name;
}

// A name as it stands inside an expression. The dollar is the only character a name may carry that
// a regular expression reads as something else, and a bracket makes it itself again in both
// dialects where a backslash is only sure in one.
function literal(name: string): string {
  return name.replace(/[$]/g, () => "[$]");
}

function expression(form: DefinitionForm, name: string): string {
  return `${form.before}${literal(name)}${form.after}`;
}

// The one expression git searches a commit with: every family's shapes at once, since the search
// cannot be narrowed to a language beforehand — the paths it would take are patterns, and git is
// run here with patterns turned off so that a path from a request is only ever a path. What comes
// back is sorted out afterwards by the file each line was found in.
export function definitionPattern(name: string): string {
  const seen = new Set<string>();

  for (const form of FORMS) {
    seen.add(expression(form, name));
  }

  return [...seen].join("|");
}

// What a line found in that file declares, or nothing where the file's own family has no shape
// that fits it — a line that matched only because another language's shape did is no definition
// here, and neither is a line in a file no family owns.
export function definitionKind(path: string, text: string, name: string): DefinitionKind | null {
  const language = definitionLanguage(path);

  if (language === null || definitionName(name, path) === null) {
    return null;
  }

  for (const form of FORMS) {
    if (form.language === language && new RegExp(expression(form, name)).test(text)) {
      return form.kind;
    }
  }

  return null;
}

// One place a name is declared, as much of it as the order below reads.
export interface DefinitionPlace {
  path: string;
  line: number;
  kind: DefinitionKind;
}

// The folders of a path, its own name dropped.
function folders(path: string): string[] {
  const parts = path.split("/");

  parts.pop();

  return parts;
}

// How near two paths stand: the folders they share from the root down.
function nearness(one: string, other: string): number {
  const mine = folders(one);
  const theirs = folders(other);
  let shared = 0;

  while (shared < mine.length && shared < theirs.length && mine[shared] === theirs[shared]) {
    shared += 1;
  }

  return shared;
}

// The order the places stand in: the reader's own file first, then a declaration before a name
// merely passed on, then the files nearest the reader's path, then the reader's own language, and
// path and line last so that one answer never comes back in a different order than the last.
export function rankDefinitions<T extends DefinitionPlace>(found: readonly T[], readerPath: string): T[] {
  const reading = definitionLanguage(readerPath);
  const mine = (place: T): number => (place.path === readerPath ? 0 : 1);
  const declared = (place: T): number => (place.kind === "re-export" ? 1 : 0);
  const tongue = (place: T): number => (reading !== null && definitionLanguage(place.path) === reading ? 0 : 1);

  return [...found].sort(
    (one, other) =>
      mine(one) - mine(other) ||
      declared(one) - declared(other) ||
      nearness(other.path, readerPath) - nearness(one.path, readerPath) ||
      tongue(one) - tongue(other) ||
      (one.path < other.path ? -1 : one.path > other.path ? 1 : 0) ||
      one.line - other.line
  );
}
