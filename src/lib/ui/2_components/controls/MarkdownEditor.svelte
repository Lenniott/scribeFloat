<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { EditorState } from '@codemirror/state';
	import { EditorView, keymap, placeholder as cmPlaceholder } from '@codemirror/view';
	import { defaultKeymap } from '@codemirror/commands';
	import { markdown } from '@codemirror/lang-markdown';

	let {
		value = $bindable(''),
		onchange
	}: { value?: string; onchange?: (v: string) => void } = $props();

	let container = $state<HTMLDivElement | null>(null);
	let view: EditorView | null = null;
	// Prevent feedback loop when we push an external value into the editor
	let updatingFromProp = false;

	const theme = EditorView.theme({
		'&': { height: '100%', background: 'var(--color-bg-canvas)', color: 'var(--color-fg)' },
		'.cm-content': {
			fontFamily: 'var(--font-sans)',
			fontSize: '0.9375rem',
			padding: '1rem'
		},
		'.cm-cursor': { borderLeftColor: 'var(--color-fg)' },
		'.cm-selectionBackground, ::selection': { background: 'var(--color-bg-active)' },
		'.cm-gutters': { display: 'none' },
		'.cm-focused': { outline: 'none' }
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
				markdown(),
				theme,
				keymap.of(defaultKeymap),
				EditorView.lineWrapping,
				cmPlaceholder('Start writing…'),
				updateListener
			]
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
					changes: { from: 0, to: view.state.doc.length, insert: value }
				})
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
