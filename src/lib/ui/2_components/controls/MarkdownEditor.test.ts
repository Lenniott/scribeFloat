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

		const announced = container.querySelector('.cm-announced') as HTMLElement | null;
		expect(announced).toBeTruthy();
		const style = getComputedStyle(announced!);
		// Theme must keep the announce region out of normal layout (sr-only style).
		expect(style.position === 'absolute' || style.clip !== 'auto').toBe(true);
	});
});
