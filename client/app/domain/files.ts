// What the code viewer says of a file beside its body: its size, in the units a reader reads at a glance, in
// the locale's digits.
export function fileSizeText(bytes: number, locale: string): string {
  if (bytes < 1024) {
    return `${bytes.toLocaleString(locale)}B`;
  }

  if (bytes < 1024 * 1024) {
    return `${Math.round(bytes / 1024).toLocaleString(locale)}KB`;
  }

  return `${(bytes / (1024 * 1024)).toLocaleString(locale, { minimumFractionDigits: 1, maximumFractionDigits: 1 })}MB`;
}
