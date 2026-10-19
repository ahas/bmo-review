//! Where a name is declared, read off the shape of the line that declares it (`docs/code-review.md`, 「Search」).
//!
//! No language server and no parser stand here: the lines that declare a name have a shape, one set of shapes per
//! language family, and git searches the commit for them. Each family states its own shapes, and two things are
//! rendered from them: the single expression git is given, and this module's own reading of a line that came
//! back, which says what kind of declaration it is and drops a line whose family does not own the shape that
//! found it.
//!
//! The same text is a regular expression in two dialects at once, the POSIX extended one git runs and this
//! crate's own, so nothing here uses what only one of them has: no word-boundary or class escape, no lookaround,
//! no lazy quantifier. A word edge is a bracket of its own, a blank is a bracket holding a space and a tab, and
//! the dollar goes through a bracket rather than a backslash.

use std::collections::BTreeSet;

use regex::Regex;
use serde::Serialize;

/// The language families a definition is looked for in. A file of any other kind declares nothing here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
  /// TypeScript and JavaScript, and the script a Vue file carries.
  Script,
  Python,
  Rust,
  CSharp,
  Shell,
  Lua,
}

/// What a line declares.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
  Function,
  Method,
  Class,
  Interface,
  Type,
  Enum,
  Const,
  Struct,
  Trait,
  Module,
  Macro,
  /// A name the file only passes on: a place the name is written, standing after every real declaration.
  ReExport,
}

/// One shape a declaration takes: what stands before the name on the line, and what follows it.
struct Form {
  kind: Kind,
  before: String,
  after: String,
}

impl Form {
  fn new(kind: Kind, before: impl Into<String>, after: impl Into<String>) -> Self {
    Self {
      kind,
      before: before.into(),
      after: after.into(),
    }
  }

  fn expression(&self, name: &str) -> String {
    format!("{}{}{}", self.before, literal(name), self.after)
  }
}

// A character that cannot be part of a name, which is how a word ends where neither dialect has a word boundary
// of its own. The blank holds a space and a real tab.
const EDGE: &str = "[^A-Za-z0-9_$]";
const BLANK: &str = "[ \t]";

fn open() -> String {
  format!("(^|{EDGE})")
}

fn close() -> String {
  format!("({EDGE}|$)")
}

fn indent() -> String {
  format!("^{BLANK}*")
}

// What may stand between the head of a line and a script declaration's own keyword. The declaration is anchored
// to the line, which keeps the `const` of a loop's head and the `type` of an import from reading as declarations.
fn script_head() -> String {
  let indent = indent();

  format!(
    "{indent}(export{BLANK}+)?(default{BLANK}+)?(declare{BLANK}+)?(abstract{BLANK}+)?(async{BLANK}+)?"
  )
}

impl Language {
  /// Every family, in the order its shapes are joined into the one expression.
  pub const ALL: [Language; 6] =
    [Self::Script, Self::Python, Self::Rust, Self::CSharp, Self::Shell, Self::Lua];

  /// The family a path belongs to by its ending, or none where no family owns it.
  pub fn of(path: &str) -> Option<Self> {
    let name = path.rsplit('/').next().unwrap_or_default().to_lowercase();
    let dot = name.rfind('.').filter(|dot| *dot > 0)?;
    let ending = &name[dot + 1..];

    Self::ALL.into_iter().find(|language| language.endings().contains(&ending))
  }

  /// The endings of a file of this family.
  fn endings(self) -> &'static [&'static str] {
    match self {
      Self::Script => &["ts", "tsx", "mts", "cts", "js", "mjs", "cjs", "jsx", "vue"],
      Self::Python => &["py", "pyi"],
      Self::Rust => &["rs"],
      Self::CSharp => &["cs"],
      Self::Shell => &["sh", "bash", "zsh"],
      Self::Lua => &["lua"],
    }
  }

  /// The words the family spends on itself: a reader pressing one points at the language, not at a name.
  fn keywords(self) -> &'static [&'static str] {
    match self {
      Self::Script => &[
        "as",
        "async",
        "await",
        "break",
        "case",
        "catch",
        "class",
        "const",
        "continue",
        "declare",
        "default",
        "delete",
        "do",
        "else",
        "enum",
        "export",
        "extends",
        "false",
        "finally",
        "for",
        "from",
        "function",
        "if",
        "implements",
        "import",
        "in",
        "instanceof",
        "interface",
        "keyof",
        "let",
        "new",
        "null",
        "of",
        "return",
        "satisfies",
        "static",
        "super",
        "switch",
        "this",
        "throw",
        "true",
        "try",
        "type",
        "typeof",
        "undefined",
        "var",
        "void",
        "while",
        "yield",
      ],
      Self::Python => &[
        "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del", "elif",
        "else", "except", "False", "finally", "for", "from", "global", "if", "import", "in", "is",
        "lambda", "None", "nonlocal", "not", "or", "pass", "raise", "return", "self", "True", "try",
        "while", "with", "yield",
      ],
      Self::Rust => &[
        "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
        "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
        "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
        "use", "where", "while",
      ],
      Self::CSharp => &[
        "abstract",
        "as",
        "async",
        "await",
        "base",
        "bool",
        "break",
        "case",
        "catch",
        "class",
        "const",
        "continue",
        "default",
        "do",
        "double",
        "else",
        "enum",
        "event",
        "false",
        "finally",
        "float",
        "for",
        "foreach",
        "if",
        "int",
        "interface",
        "internal",
        "is",
        "lock",
        "namespace",
        "new",
        "null",
        "object",
        "override",
        "partial",
        "private",
        "protected",
        "public",
        "readonly",
        "record",
        "ref",
        "return",
        "sealed",
        "static",
        "string",
        "struct",
        "switch",
        "this",
        "throw",
        "true",
        "try",
        "typeof",
        "using",
        "var",
        "virtual",
        "void",
        "while",
      ],
      Self::Shell => &[
        "case", "declare", "do", "done", "elif", "else", "esac", "export", "fi", "for", "function", "if",
        "in", "local", "readonly", "return", "then", "typeset", "until", "while",
      ],
      Self::Lua => &[
        "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "if", "in", "local",
        "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
      ],
    }
  }

  /// Every shape a declaration of this family takes, the more particular before the looser, since the first
  /// that fits a line names what that line declares.
  fn forms(self) -> Vec<Form> {
    let (open, close, indent) = (open(), close(), indent());

    match self {
      Self::Script => {
        let head = script_head();

        vec![
          Form::new(
            Kind::Function,
            format!("{head}function({BLANK}+[*]?{BLANK}*|{BLANK}*[*]{BLANK}*)"),
            format!("{BLANK}*[(<]"),
          ),
          Form::new(Kind::Class, format!("{head}class{BLANK}+"), close.clone()),
          Form::new(Kind::Interface, format!("{head}interface{BLANK}+"), close.clone()),
          Form::new(Kind::Enum, format!("{head}(const{BLANK}+)?enum{BLANK}+"), close.clone()),
          Form::new(Kind::Type, format!("{head}type{BLANK}+"), format!("{BLANK}*[=<]")),
          Form::new(Kind::Const, format!("{head}(const|let|var){BLANK}+"), format!("{BLANK}*[:=]")),
          Form::new(
            Kind::ReExport,
            format!("{indent}export{BLANK}*(type{BLANK}+)?[{{]([^}}]*{EDGE})?"),
            close,
          ),
        ]
      }
      // A method is written exactly as a function is; a module's own constant is an assignment at the very head
      // of a line, which separates it from the bindings inside a body.
      Self::Python => vec![
        Form::new(Kind::Function, format!("{indent}(async{BLANK}+)?def{BLANK}+"), format!("{BLANK}*[(]")),
        Form::new(Kind::Class, format!("{indent}class{BLANK}+"), format!("{BLANK}*[(:]")),
        Form::new(Kind::Const, "^", format!("{BLANK}*[:=]")),
      ],
      // Every keyword here is reserved, so a shape is anchored to the keyword rather than to the line.
      Self::Rust => vec![
        Form::new(Kind::Function, format!("{open}fn{BLANK}+"), format!("{BLANK}*[(<]")),
        Form::new(Kind::Struct, format!("{open}struct{BLANK}+"), close.clone()),
        Form::new(Kind::Enum, format!("{open}enum{BLANK}+"), close.clone()),
        Form::new(Kind::Trait, format!("{open}trait{BLANK}+"), close.clone()),
        Form::new(Kind::Type, format!("{open}type{BLANK}+"), format!("{BLANK}*[=<]")),
        Form::new(
          Kind::Const,
          format!("{open}(const|static){BLANK}+(mut{BLANK}+)?"),
          format!("{BLANK}*[:]"),
        ),
        Form::new(Kind::Module, format!("{open}mod{BLANK}+"), close.clone()),
        Form::new(Kind::Macro, format!("{open}macro_rules!{BLANK}*"), close),
      ],
      // A method carries no keyword of its own, so it is known by the modifiers its line opens with and the
      // brackets after the name; stopping before an equals or a semicolon keeps a call from reading as one.
      Self::CSharp => vec![
        Form::new(Kind::Class, format!("{open}class{BLANK}+"), close.clone()),
        Form::new(Kind::Class, format!("{open}record{BLANK}+"), close.clone()),
        Form::new(Kind::Struct, format!("{open}struct{BLANK}+"), close.clone()),
        Form::new(Kind::Interface, format!("{open}interface{BLANK}+"), close.clone()),
        Form::new(Kind::Enum, format!("{open}enum{BLANK}+"), close),
        Form::new(
          Kind::Method,
          format!(
            "{indent}(public|private|protected|internal|static|async|override|virtual|sealed|abstract|partial|new)[^=;]*{EDGE}"
          ),
          format!("{BLANK}*[(<]"),
        ),
      ],
      // A function is a name with an empty pair of brackets after it, with or without the word in front; a
      // variable is an assignment at the head of its line.
      Self::Shell => vec![
        Form::new(
          Kind::Function,
          format!("{indent}(function{BLANK}+)?"),
          format!("{BLANK}*[(]{BLANK}*[)]"),
        ),
        Form::new(Kind::Function, format!("{indent}function{BLANK}+"), format!("({BLANK}|[{{]|$)")),
        Form::new(
          Kind::Const,
          format!(
            "{indent}(export{BLANK}+|declare{BLANK}+|local{BLANK}+|readonly{BLANK}+|typeset{BLANK}+)?"
          ),
          "=",
        ),
      ],
      Self::Lua => vec![
        Form::new(
          Kind::Function,
          format!("{open}(local{BLANK}+)?function{BLANK}+"),
          format!("{BLANK}*[(]"),
        ),
        Form::new(Kind::Const, format!("{open}local{BLANK}+"), format!("{BLANK}*=")),
      ],
    }
  }
}

// What a name may look like at all: two characters at least, since one finds half the repository, and an upper
// bound no identifier comes near, so the expression handed to git stays a known size.
const NAME_LEAST: usize = 2;
const NAME_MOST: usize = 128;

fn is_name_shape(name: &str) -> bool {
  let mut characters = name.chars();

  characters
    .next()
    .is_some_and(|first| first.is_ascii_alphabetic() || first == '_' || first == '$')
    && characters.all(|rest| rest.is_ascii_alphanumeric() || rest == '_' || rest == '$')
}

/// The word a reader pressed, as a name worth looking for, or none where it is not one. In a file of no family
/// only the words every family spends on itself are refused.
pub fn definition_name(word: &str, path: &str) -> Option<String> {
  let name = bmo_markup::js::trim(word);
  let length = name.len();

  if !is_name_shape(name) || !(NAME_LEAST..=NAME_MOST).contains(&length) {
    return None;
  }

  let spent = match Language::of(path) {
    Some(language) => language.keywords().contains(&name),
    None => Language::ALL.iter().any(|language| language.keywords().contains(&name)),
  };

  (!spent).then(|| name.to_owned())
}

// A name as it stands inside an expression: the dollar is the one character a name may carry that a regular
// expression reads as something else, and a bracket makes it itself again in both dialects.
fn literal(name: &str) -> String {
  name.replace('$', "[$]")
}

/// The one expression git searches a commit with: every family's shapes at once, each once.
pub fn definition_pattern(name: &str) -> String {
  let mut seen = BTreeSet::new();
  let mut joined = Vec::new();

  for form in Language::ALL.into_iter().flat_map(Language::forms) {
    let expression = form.expression(name);

    if seen.insert(expression.clone()) {
      joined.push(expression);
    }
  }

  joined.join("|")
}

/// What a line found in that file declares, or none where the file's own family has no shape that fits it.
pub fn definition_kind(path: &str, text: &str, name: &str) -> Option<Kind> {
  let language = Language::of(path)?;

  definition_name(name, path)?;

  language.forms().into_iter().find_map(|form| {
    Regex::new(&form.expression(name))
      .ok()
      .filter(|shape| shape.is_match(text))
      .map(|_| form.kind)
  })
}

/// One place a name is declared.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Definition {
  pub path: String,
  pub line: u64,
  pub text: String,
  pub kind: Kind,
}

/// How near two paths stand: the folders they share from the root down.
fn nearness(one: &str, other: &str) -> usize {
  let folders = |path: &str| {
    let mut parts: Vec<String> = path.split('/').map(str::to_owned).collect();

    parts.pop();
    parts
  };
  let (mine, theirs) = (folders(one), folders(other));

  mine.iter().zip(&theirs).take_while(|(a, b)| a == b).count()
}

/// The order the places stand in: the reader's own file first, then a declaration before a name merely passed
/// on, then the files nearest the reader's path, then the reader's own language, then path and line.
pub fn rank(found: &mut [Definition], reader: &str) {
  let reading = Language::of(reader);
  let mine = |place: &Definition| u8::from(place.path != reader);
  let declared = |place: &Definition| u8::from(place.kind == Kind::ReExport);
  let tongue = |place: &Definition| u8::from(reading.is_none() || Language::of(&place.path) != reading);

  found.sort_by(|one, other| {
    mine(one)
      .cmp(&mine(other))
      .then_with(|| declared(one).cmp(&declared(other)))
      .then_with(|| nearness(&other.path, reader).cmp(&nearness(&one.path, reader)))
      .then_with(|| tongue(one).cmp(&tongue(other)))
      .then_with(|| one.path.cmp(&other.path))
      .then_with(|| one.line.cmp(&other.line))
  });
}
