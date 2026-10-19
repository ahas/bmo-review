// What the code viewer says of a file beside its body: its size, and whether it has a rendered form to read
// it in as well as its source.

// The endings of a file there is a renderer for. Everything else is code, and code has no second form.
const RENDERED_FORMATS = new Set(["md", "markdown"]);

function extension(path: string): string {
  const name = path.slice(path.lastIndexOf("/") + 1);
  const dot = name.lastIndexOf(".");

  return dot <= 0 ? "" : name.slice(dot + 1).toLowerCase();
}

// Whether a file is one there is something other than its source to look at.
export function hasRenderedForm(path: string): boolean {
  return RENDERED_FORMATS.has(extension(path));
}

// A file's size in the units a reader reads at a glance, in the locale's digits.
export function fileSizeText(bytes: number, locale: string): string {
  if (bytes < 1024) {
    return `${bytes.toLocaleString(locale)}B`;
  }

  if (bytes < 1024 * 1024) {
    return `${Math.round(bytes / 1024).toLocaleString(locale)}KB`;
  }

  return `${(bytes / (1024 * 1024)).toLocaleString(locale, { minimumFractionDigits: 1, maximumFractionDigits: 1 })}MB`;
}
