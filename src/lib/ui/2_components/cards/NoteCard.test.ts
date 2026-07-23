import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import { historyFixtures } from '../../../../test/ipcFixtures';
import NoteCard from './NoteCard.svelte';

describe('NoteCard', () => {
	const item = historyFixtures.storeScribe();

	it('fires onselect when title is clicked', async () => {
		const onselect = vi.fn();
		render(NoteCard, {
			props: {
				item,
				chip: { label: 'Scribe', variant: 'brand' },
				onselect,
			},
		});

		await fireEvent.click(screen.getByRole('button', { name: item.title }));
		expect(onselect).toHaveBeenCalledOnce();
	});

	it('fires oncopy from copy action', async () => {
		const oncopy = vi.fn();
		render(NoteCard, {
			props: {
				item,
				chip: { label: 'Scribe', variant: 'brand' },
				oncopy,
			},
		});

		await fireEvent.click(screen.getByRole('button', { name: 'Copy to clipboard' }));
		expect(oncopy).toHaveBeenCalledOnce();
	});

	it('omits delete action when ondelete is undefined', () => {
		render(NoteCard, {
			props: {
				item,
				chip: { label: 'Scribe', variant: 'brand' },
			},
		});

		expect(screen.queryByRole('button', { name: 'Delete note' })).not.toBeInTheDocument();
	});

	it('hides delete for legacy items', () => {
		const legacy = historyFixtures.legacyDictate();
		render(NoteCard, {
			props: {
				item: legacy,
				chip: { label: 'Dictate', variant: 'muted' },
				ondelete: vi.fn(),
				onopen: vi.fn(),
			},
		});

		expect(screen.queryByRole('button', { name: 'Delete note' })).not.toBeInTheDocument();
		expect(screen.getByText('Legacy')).toBeInTheDocument();
	});
});
