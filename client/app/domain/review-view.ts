// The editor the code review shows a file in: bmo-ui's read-only editor with a margin a comment is written
// from and marked in (docs/screen.md, 「Reviewing」). Importing bmo-ui's editor module first sets up what
// every editor of the layer stands on — the worker, the tokenizers, the diff language — and this module
// adds only the review's own view on top of it. Like bmo-ui's editor, it is reached from the browser alone,
// with a dynamic import once the code section is on the screen.
//
// The service's module carries an editor of its own beside the console's, since the editor is no library the
// console shares; bmo-ui's module sets up the console's editor, and the worker it names is the one every editor
// of the page starts, so what this module draws with is given the layer's two themes and every tokenizer here.

import * as monaco from "monaco-editor/editor/editor.api";

import "monaco-editor/languages/definitions/register.all.js";

import { CODE_THEMES } from "bmo-ui/app/domain/code-themes";
import { languageOf } from "bmo-ui/app/domain/monaco";

import type { Translate } from "bmo-ui/shared/wording";

for (const { name, theme } of Object.values(CODE_THEMES)) {
  monaco.editor.defineTheme(name, theme);
}

function themeName(dark: boolean): string {
  return CODE_THEMES[dark ? "dark" : "light"].name;
}

// A text put into the editor as a model of its own, and the model it replaces thrown away once it no
// longer stands there. What comes back is the model now shown.
function replaceModel(
  editor: monaco.editor.IStandaloneCodeEditor,
  previous: monaco.editor.ITextModel | null,
  text: string,
  language: string
): monaco.editor.ITextModel {
  const made = monaco.editor.createModel(text, language);

  editor.setModel(made);
  previous?.dispose();

  return made;
}

// How the file stands, as bmo-ui's code view has it: read-only twice over, stripped of everything an editor
// does for somebody writing, measured on the console's fixed-width face at text-sm.
const VIEW: monaco.editor.IStandaloneEditorConstructionOptions = {
  readOnly: true,
  domReadOnly: true,
  automaticLayout: true,
  fontFamily: "var(--font-mono)",
  fontSize: 14,
  lineHeight: 20,
  lineNumbers: "on",
  minimap: { enabled: false },
  contextmenu: false,
  quickSuggestions: false,
  suggestOnTriggerCharacters: false,
  renderLineHighlight: "line",
  overviewRulerLanes: 0,
  overviewRulerBorder: false,
  scrollBeyondLastLine: false,
  scrollbar: { useShadows: false },
  padding: { top: 4, bottom: 4 }
};

// The line the file was opened at, marked across the whole of it in the token the console raises a row in.
const OPEN_LINE: monaco.editor.IModelDecorationOptions = {
  isWholeLine: true,
  className: "bg-elevated",
  marginClassName: "bg-elevated"
};


// A file as the code review shows it (docs/screen.md): the same read-only editor, with a margin a comment is
// written from and marked in. A line is chosen by pressing its number or the margin beside it, and a run of
// lines by pressing a second one; lines are chosen by dragging too. A line comment at the revision shown marks
// its first line in the margin and tints its lines, and anything the screen wants between the lines (a comment
// folded to one cell, the box a new comment is written in) is a zone the screen fills.
export interface CommentMark {
  id: number;
  start: number;
  end: number;
}

export interface CommentZone {
  key: string;
  afterLine: number;
}

export interface LineSelection {
  start: number;
  end: number;
  // Where the end of the selection stands on the screen, for a button to be put beside it.
  rect: { top: number; left: number; height: number };
}

// Where a press or the cursor stands in the code: the line, and the name under it where there is
// one. What counts as one name is the editor's own reading of the language it opened the file in,
// not a rule of the console's, so a press on `openReviewView` carries that whole word and a press
// on the bracket after it carries none.
export interface CodePoint {
  line: number;
  word: string | null;
  // Where the name stands on the screen, so that what the console puts beside it — a list of the
  // places it is declared — stands at the name rather than at the middle of the panel.
  rect: { top: number; left: number; height: number };
}

export interface ReviewView {
  // A file's text, coloured in the language its path says, opened at a line.
  show: (text: string, path: string, line: number | null) => void;
  paint: (dark: boolean) => void;
  // The comments standing on the file, and the lines being chosen for a new one.
  mark: (marks: readonly CommentMark[], chosen: { start: number; end: number } | null) => void;
  // The zones between the lines, replacing those before; each comes back as the element to fill.
  zones: (zones: readonly CommentZone[]) => Map<string, HTMLElement>;
  // A zone's height, once what fills it has been measured.
  resize: (key: string, height: number) => void;
  reveal: (line: number) => void;
  // The line at the top of the view, and a view put back with that line at its top.
  topLine: () => number;
  setTopLine: (line: number) => void;
  lineCount: () => number;
  // A line pressed in the margin, and whether the press asked to widen what was chosen.
  onLine: (listener: (line: number, widen: boolean) => void) => void;
  // A comment's mark pressed.
  onMark: (listener: (id: number) => void) => void;
  onSelection: (listener: (selection: LineSelection | null) => void) => void;
  // What a press and the cursor stand on, for go-to-definition: read on demand where the screen has a
  // point of its own to ask about, and told to the screen as the cursor moves so that an action
  // can say whether there is anything to go to.
  pointAt: (x: number, y: number) => CodePoint | null;
  pointAtCursor: () => CodePoint | null;
  onCursor: (listener: (point: CodePoint | null) => void) => void;
  // Go-to-definition asked from the keyboard, which is the editor's own F12.
  onDefinition: (listener: (point: CodePoint) => void) => void;
  close: () => void;
}

const MARK_START: monaco.editor.IModelDecorationOptions = {
  glyphMarginClassName: "code-review-glyph-mark"
};

const MARK_LINES: monaco.editor.IModelDecorationOptions = {
  isWholeLine: true,
  className: "code-review-lines",
  marginClassName: "code-review-lines"
};

const CHOSEN_LINES: monaco.editor.IModelDecorationOptions = {
  isWholeLine: true,
  className: "code-review-chosen",
  marginClassName: "code-review-chosen"
};

const ADD_HERE: monaco.editor.IModelDecorationOptions = {
  glyphMarginClassName: "code-review-glyph-add"
};

// Where what a zone holds (a comment's cell, or the box a new comment is written in) stands until the zone
// reports its place: far above the editor, so nothing unplaced is drawn over the code.
const UNPLACED_ZONE_TOP = "-10000px";

// How tall a zone opens before what it holds has been measured; the measured height replaces it.
const OPENING_ZONE_HEIGHT_PX = 40;

interface HeldZone {
  id: string;
  zone: monaco.editor.IViewZone;
  widget: monaco.editor.IOverlayWidget;
  content: HTMLElement;
}

export function openReviewView(t: Translate, box: HTMLElement, dark: boolean): ReviewView {
  // The two marks in the margin, each with the words its hover says.
  const markStart = { ...MARK_START, glyphMarginHoverMessage: { value: t("code.action.open_comment") } };
  const addHere = { ...ADD_HERE, glyphMarginHoverMessage: { value: t("code.action.add_comment") } };

  const editor = monaco.editor.create(box, {
    ...VIEW,
    theme: themeName(dark),
    glyphMargin: true,
    selectionHighlight: false,
    occurrencesHighlight: "off"
  });
  const opened = editor.createDecorationsCollection();
  const marked = editor.createDecorationsCollection();
  const hovered = editor.createDecorationsCollection();
  const lineListeners: Array<(line: number, widen: boolean) => void> = [];
  const markListeners: Array<(id: number) => void> = [];
  const definitionListeners: Array<(point: CodePoint) => void> = [];
  let model: monaco.editor.ITextModel | null = null;
  let starts = new Map<number, number>();
  let zoneIds = new Map<string, HeldZone>();
  let opening: number | null = null;
  let shown = false;

  function reveal(measure: boolean): void {
    if (opening === null || shown || box.clientHeight === 0) {
      return;
    }

    if (measure) {
      editor.layout();
    }

    editor.revealLineInCenter(opening);
    shown = true;
  }

  // What fills a zone is as wide as the text area and starts where the text does.
  function fitZones(): void {
    const layout = editor.getLayoutInfo();

    for (const { content } of zoneIds.values()) {
      content.style.left = `${layout.contentLeft}px`;
      content.style.width = `${layout.contentWidth - layout.verticalScrollbarWidth}px`;
    }
  }

  editor.onDidLayoutChange(() => {
    reveal(false);
    fitZones();
  });

  const inMargin = (type: monaco.editor.MouseTargetType): boolean =>
    type === monaco.editor.MouseTargetType.GUTTER_GLYPH_MARGIN || type === monaco.editor.MouseTargetType.GUTTER_LINE_NUMBERS;

  // The plus follows the pointer down the margin, so what a press there would do is said before it.
  editor.onMouseMove((event) => {
    const line = event.target.position?.lineNumber;

    hovered.set(
      inMargin(event.target.type) && line !== undefined && !starts.has(line)
        ? [{ range: new monaco.Range(line, 1, line, 1), options: addHere }]
        : []
    );
  });

  editor.onMouseLeave(() => hovered.clear());

  function pointOf(position: monaco.IPosition | null | undefined): CodePoint | null {
    if (position === null || position === undefined || model === null) {
      return null;
    }

    // Where the name is on the screen. A position scrolled out of sight has no place of its own,
    // and the panel's own corner stands for it.
    const where = editor.getScrolledVisiblePosition(position);
    const frame = box.getBoundingClientRect();

    return {
      line: position.lineNumber,
      word: model.getWordAtPosition(position)?.word ?? null,
      rect:
        where === null
          ? { top: frame.top, left: frame.left, height: 0 }
          : { top: frame.top + where.top, left: frame.left + where.left, height: where.height }
    };
  }

  // Go-to-definition from the keyboard. An action added to the editor carries its own keybinding with
  // nothing else loaded, and F12 is the key the editor itself keeps for this. It is given no
  // context menu group: the menu that stands beside add-comment is the console's own, and a group
  // named here would only register an entry nothing draws.
  editor.addAction({
    id: "bmo-go-to-definition",
    label: t("code.action.go_to_definition"),
    keybindings: [monaco.KeyCode.F12],
    run: () => {
      const point = pointOf(editor.getPosition());

      if (point !== null && point.word !== null) {
        definitionListeners.forEach((listener) => listener(point));
      }
    }
  });

  editor.onMouseDown((event) => {
    const line = event.target.position?.lineNumber;

    if (line === undefined || !inMargin(event.target.type)) {
      return;
    }

    const comment = starts.get(line);

    if (comment !== undefined && event.target.type === monaco.editor.MouseTargetType.GUTTER_GLYPH_MARGIN) {
      markListeners.forEach((listener) => listener(comment));

      return;
    }

    lineListeners.forEach((listener) => listener(line, event.event.shiftKey));
  });

  return {
    show(text: string, path: string, line: number | null): void {
      model = replaceModel(editor, model, text, languageOf(path));
      opening = line;
      shown = false;
      opened.set(line === null ? [] : [{ range: new monaco.Range(line, 1, line, 1), options: OPEN_LINE }]);
      reveal(true);
    },

    paint(dark: boolean): void {
      monaco.editor.setTheme(themeName(dark));
    },

    mark(marks, chosen): void {
      const count = model?.getLineCount() ?? 0;
      const inside = (line: number): number => Math.min(Math.max(1, line), Math.max(1, count));
      const decorations: monaco.editor.IModelDeltaDecoration[] = [];

      starts = new Map();

      for (const one of marks) {
        const start = inside(one.start);
        const end = inside(one.end);

        if (!starts.has(start)) {
          starts.set(start, one.id);
          decorations.push({ range: new monaco.Range(start, 1, start, 1), options: markStart });
        }

        decorations.push({ range: new monaco.Range(start, 1, end, 1), options: MARK_LINES });
      }

      if (chosen !== null) {
        decorations.push({ range: new monaco.Range(inside(chosen.start), 1, inside(chosen.end), 1), options: CHOSEN_LINES });
      }

      marked.set(decorations);
    },

    zones(zones): Map<string, HTMLElement> {
      const nodes = new Map<string, HTMLElement>();
      const next = new Map<string, HeldZone>();

      editor.changeViewZones((accessor) => {
        for (const held of zoneIds.values()) {
          accessor.removeZone(held.id);
          editor.removeOverlayWidget(held.widget);
        }

        for (const wanted of zones) {
          // A zone only keeps the room open: the editor draws its lines over its zones and takes
          // every press there for itself. What fills the room is a widget laid over the lines,
          // moved to wherever the zone is each time the editor draws it.
          const content = document.createElement("div");
          const widget: monaco.editor.IOverlayWidget = {
            getId: () => `code-review-zone-${wanted.key}`,
            getDomNode: () => content,
            getPosition: () => null
          };
          const zone: monaco.editor.IViewZone = {
            afterLineNumber: wanted.afterLine,
            heightInPx: OPENING_ZONE_HEIGHT_PX,
            domNode: document.createElement("div"),
            onDomNodeTop: (top) => {
              content.style.top = `${top}px`;
            },
            onComputedHeight: (height) => {
              content.style.maxHeight = `${height}px`;
            }
          };

          content.style.position = "absolute";
          content.style.top = UNPLACED_ZONE_TOP;
          editor.addOverlayWidget(widget);
          next.set(wanted.key, { id: accessor.addZone(zone), zone, widget, content });
          nodes.set(wanted.key, content);
        }
      });

      zoneIds = next;
      fitZones();

      return nodes;
    },

    resize(key, height): void {
      const held = zoneIds.get(key);

      if (held === undefined || held.zone.heightInPx === height) {
        return;
      }

      held.zone.heightInPx = height;
      editor.changeViewZones((accessor) => accessor.layoutZone(held.id));
    },

    // The line the address now names: marked as the line a file is opened at is marked, and
    // brought into view where it is not already there. A jump inside one file is that and nothing
    // more, since the file it lands in is the file already open.
    reveal(line): void {
      opened.set([{ range: new monaco.Range(line, 1, line, 1), options: OPEN_LINE }]);
      editor.revealLineInCenterIfOutsideViewport(line);
    },

    topLine: () => editor.getVisibleRanges()[0]?.startLineNumber ?? 1,

    setTopLine(line): void {
      editor.setScrollTop(editor.getTopForLineNumber(Math.max(1, line)), monaco.editor.ScrollType.Immediate);
    },

    lineCount: () => model?.getLineCount() ?? 0,

    onLine(listener): void {
      lineListeners.push(listener);
    },

    onMark(listener): void {
      markListeners.push(listener);
    },

    pointAt: (x, y) => pointOf(editor.getTargetAtClientPoint(x, y)?.position),

    pointAtCursor: () => pointOf(editor.getPosition()),

    onCursor(listener): void {
      editor.onDidChangeCursorPosition((event) => listener(pointOf(event.position)));
    },

    onDefinition(listener): void {
      definitionListeners.push(listener);
    },

    onSelection(listener): void {
      editor.onDidChangeCursorSelection((event) => {
        const selection = event.selection;
        const end = editor.getScrolledVisiblePosition(selection.getEndPosition());

        if (selection.isEmpty() || end === null) {
          listener(null);

          return;
        }

        // A drag that ends at the very start of a line has not taken anything of that line.
        const last = selection.endColumn === 1 && selection.endLineNumber > selection.startLineNumber ? selection.endLineNumber - 1 : selection.endLineNumber;
        const frame = box.getBoundingClientRect();

        listener({
          start: selection.startLineNumber,
          end: last,
          rect: { top: frame.top + end.top, left: frame.left + end.left, height: end.height }
        });
      });
    },

    close(): void {
      editor.dispose();
      model?.dispose();
    }
  };
}
