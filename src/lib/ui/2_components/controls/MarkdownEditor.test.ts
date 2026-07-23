import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { render, waitFor } from '@testing-library/svelte';
import { EditorView } from '@codemirror/view';
import { describe, expect, it, vi } from 'vitest';
import MarkdownEditor from './MarkdownEditor.svelte';

describe('MarkdownEditor', () => {
	it('does not put Selection deleted into the document on range delete', async () => {
		const onchange = vi.fn();
		const { container } = render(MarkdownEditor, {
			props: { value: 'hello world', onchange },
		});

		await waitFor(() => {
			expect(container.querySelector('.cm-editor')).toBeTruthy();
		});

		const cmContent = container.querySelector('.cm-content') as HTMLElement | null;
		expect(cmContent).toBeTruthy();

		const view = EditorView.findFromDOM(cmContent!);
		expect(view).toBeTruthy();

		view!.dispatch({
			selection: { anchor: 0, head: view!.state.doc.length },
		});
		view!.dispatch(view!.state.replaceSelection(''));

		expect(view!.state.doc.toString()).toBe('');
		expect(view!.state.doc.toString()).not.toContain('Selection deleted');
	});

	// jsdom never loads app.css, so the layout of .cm-announced cannot be
	// asserted through the component. Guard the stylesheet source instead:
	// ticket 21 regressed exactly here (announce region painting as content).
	it('app.css keeps the announce region sr-only without removing it from the a11y tree', () => {
		const css = readFileSync(resolve(__dirname, '../../../../app.css'), 'utf8');
		const rule = css.match(/\.cm-announced\s*\{([^}]*)\}/);
		expect(rule, 'app.css must style .cm-announced').toBeTruthy();
		const body = rule![1];
		// display:none would strip it from the accessibility tree entirely.
		expect(body).not.toMatch(/display\s*:\s*none/);
		// Must be clipped out of layout (sr-only pattern).
		expect(body).toMatch(/position\s*:\s*absolute/);
		expect(body).toMatch(/overflow\s*:\s*hidden/);
	});
});
