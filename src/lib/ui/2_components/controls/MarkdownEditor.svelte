<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { EditorState } from "@codemirror/state";
  import {
    EditorView,
    drawSelection,
    keymap,
    placeholder as cmPlaceholder,
  } from "@codemirror/view";
  import { defaultKeymap } from "@codemirror/commands";
  import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
  import { markdown, markdownLanguage } from "@codemirror/lang-markdown";
  import { tags } from "@lezer/highlight";

  let {
    value = $bindable(""),
    onchange,
  }: { value?: string; onchange?: (v: string) => void } = $props();

  let container = $state<HTMLDivElement | null>(null);
  let view: EditorView | null = null;
  // Prevent feedback loop when we push an external value into the editor
  let updatingFromProp = false;

  const markdownHighlightStyle = HighlightStyle.define([
    { tag: tags.heading1, class: "cmt-heading1" },
    { tag: tags.heading2, class: "cmt-heading2" },
    { tag: tags.heading3, class: "cmt-heading3" },
    { tag: tags.heading4, class: "cmt-heading4" },
    { tag: tags.heading5, class: "cmt-heading5" },
    { tag: tags.heading6, class: "cmt-heading6" },
    { tag: tags.strong, class: "cmt-strong" },
    { tag: tags.emphasis, class: "cmt-emphasis" },
    { tag: tags.strikethrough, class: "cmt-strikethrough" },
    { tag: tags.monospace, class: "cmt-monospace" },
    { tag: tags.quote, class: "cmt-quote" },
    { tag: tags.link, class: "cmt-link" },
    { tag: tags.url, class: "cmt-link" },
    { tag: tags.processingInstruction, class: "cmt-processingInstruction" },
    { tag: tags.comment, class: "cmt-meta" },
    { tag: tags.meta, class: "cmt-meta" },
  ]);

  const selectionTheme = EditorView.baseTheme({
    "&light .cm-selectionBackground, &dark .cm-selectionBackground, &light.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, &dark.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground":
      {
        background:
          "color-mix(in oklch, var(--color-active) 18%, var(--color-fill))",
      },
  });

  const theme = EditorView.theme({
    "&": {
      height: "100%",
      background: "var(--color-canvas)",
      color: "var(--color-fg)",
    },
    ".cm-scroller": {
      fontFamily: "var(--font-sans)",
      fontSize: "0.9375rem",
      lineHeight: 1.4,
      padding: "1rem",
    },
    ".cm-content": {
      caretColor: "var(--color-fg)",
    },
    ".cm-line": {
      padding: 0,
    },
    ".cm-cursor, .cm-dropCursor": {
      borderLeft: "1.2px solid var(--color-fg)",
    },
    ".cm-gutters": { display: "none" },
    "&.cm-focused": { outline: "none", boxShadow: "none" },
    ".cm-content:focus": { outline: "none", boxShadow: "none" },
  });

  const updateListener = EditorView.updateListener.of((update) => {
    if (!update.docChanged) return;
    if (updatingFromProp) return;
    const text = update.state.doc.toString();
    value = text;
    onchange?.(text);
  });

  onMount(() => {
    if (!container) return;

    const state = EditorState.create({
      doc: value,
      extensions: [
        markdown({ base: markdownLanguage }),
        syntaxHighlighting(markdownHighlightStyle),
        selectionTheme,
        theme,
        drawSelection(),
        keymap.of(defaultKeymap),
        EditorView.lineWrapping,
        cmPlaceholder("Start writing…"),
        updateListener,
      ],
    });

    view = new EditorView({ state, parent: container });
  });

  // Push external value changes (e.g. initial load from backend) into the editor
  $effect(() => {
    if (!view) return;
    const current = view.state.doc.toString();
    if (current !== value) {
      updatingFromProp = true;
      view.dispatch(
        view.state.update({
          changes: { from: 0, to: view.state.doc.length, insert: value },
        }),
      );
      updatingFromProp = false;
    }
  });

  onDestroy(() => {
    view?.destroy();
    view = null;
  });
</script>

<div bind:this={container} class="h-full w-full"></div>
