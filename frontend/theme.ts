import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { EditorView } from "@codemirror/view";
import { tags as t } from "@lezer/highlight";

/** Matte dark surfaces with Rektoff green; mirrors the custom properties in style.css. */
export const palette = {
  bg: "#0d100e",
  panel: "#131714",
  raised: "#1a1f1b",
  line: "#262d28",
  text: "#dde3de",
  muted: "#98a39b",
  faint: "#5d6a61",
  accent: "#00f800",
  accentSoft: "#3ddc5f",
  accentWash: "rgba(0, 248, 0, 0.06)",
  fail: "#f2686a",
};

const mono = '"IBM Plex Mono", ui-monospace, monospace';

/** Editor chrome: background, gutter, cursor, selection, tooltips. */
const chrome = EditorView.theme(
  {
    "&": { backgroundColor: palette.bg, color: palette.text },
    ".cm-content": { fontFamily: mono, caretColor: palette.accent },
    ".cm-scroller": { fontFamily: mono, lineHeight: "1.6" },
    ".cm-cursor, .cm-dropCursor": { borderLeftColor: palette.accent, borderLeftWidth: "2px" },
    "&.cm-focused": { outline: "none" },
    "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection":
      { backgroundColor: "rgba(61, 220, 95, 0.22)" },
    ".cm-gutters": {
      backgroundColor: palette.bg,
      color: palette.faint,
      border: "none",
      borderRight: `1px solid ${palette.line}`,
    },
    ".cm-activeLine": { backgroundColor: palette.accentWash },
    ".cm-activeLineGutter": { backgroundColor: palette.accentWash, color: palette.text },
    ".cm-matchingBracket, &.cm-focused .cm-matchingBracket": {
      backgroundColor: "rgba(61, 220, 95, 0.18)",
      outline: "none",
    },
    ".cm-selectionMatch": { backgroundColor: "rgba(61, 220, 95, 0.1)" },
    ".cm-foldPlaceholder": {
      backgroundColor: palette.raised,
      border: `1px solid ${palette.line}`,
      color: palette.muted,
    },
    ".cm-tooltip": {
      backgroundColor: palette.raised,
      border: `1px solid ${palette.line}`,
      borderRadius: "4px",
      color: palette.text,
      boxShadow: "0 8px 24px rgba(0, 0, 0, 0.45)",
      overflow: "hidden",
    },
    ".cm-tooltip.cm-tooltip-autocomplete > ul": { fontFamily: mono, fontSize: "13px" },
    ".cm-tooltip.cm-tooltip-autocomplete > ul > li": { padding: "3px 10px" },
    ".cm-tooltip-autocomplete ul li[aria-selected]": {
      backgroundColor: "rgba(61, 220, 95, 0.18)",
      color: palette.text,
    },
    ".cm-completionDetail": { color: palette.faint, fontStyle: "normal", marginLeft: "1em" },
    ".cm-completionMatchedText": { textDecoration: "none", color: palette.accentSoft },
    ".cm-completionIcon": { display: "none" },
    ".cm-snippetField": { backgroundColor: palette.accentWash },
    ".cm-panels": { backgroundColor: palette.panel, color: palette.text },
    ".cm-searchMatch": { backgroundColor: "rgba(61, 220, 95, 0.25)" },
  },
  { dark: true },
);

/**
 * Syntax colours: neutral text with a restrained green family. Keywords carry
 * the brand green, comments recede, and `#[account(...)]` attributes and
 * macros get a muted teal because they are where the security-relevant
 * constraints live.
 */
const highlight = HighlightStyle.define([
  {
    tag: [t.keyword, t.controlKeyword, t.definitionKeyword, t.moduleKeyword, t.modifier],
    color: palette.accentSoft,
  },
  { tag: [t.self, t.bool, t.atom], color: palette.accentSoft },
  { tag: [t.comment, t.lineComment, t.blockComment], color: palette.faint, fontStyle: "italic" },
  { tag: [t.string, t.character, t.special(t.string)], color: "#c5d8a4" },
  { tag: [t.number, t.integer, t.float], color: "#d9c88f" },
  { tag: [t.typeName, t.className, t.namespace], color: "#9fe3b0" },
  { tag: [t.attributeName, t.meta, t.macroName], color: "#62cfae" },
  { tag: [t.function(t.variableName), t.function(t.propertyName)], color: "#f1f5f2" },
  { tag: [t.operator, t.punctuation, t.bracket, t.separator], color: palette.muted },
  { tag: t.labelName, color: palette.muted, fontStyle: "italic" },
  { tag: t.invalid, color: palette.fail },
]);

export const rektoffTheme = [chrome, syntaxHighlighting(highlight)];
