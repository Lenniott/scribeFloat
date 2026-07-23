import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import TranscriptPanel from './TranscriptPanel.svelte';

vi.mock('@tauri-apps/plugin-clipboard-manager', () => ({
	writeText: vi.fn().mockResolvedValue(undefined),
}));

const mockedInvoke = vi.mocked(invoke);

type Detail = Record<string, unknown>;

function detailFixture(): Detail {
	return {
		notes: [],
		speaker_blocks: [
			{ label: 'Speaker 1', start_ms: 0, end_ms: 10_000, text: 'first block' },
			{ label: 'Speaker 2', start_ms: 10_000, end_ms: 20_000, text: 'second block' },
		],
	};
}

function relabeledDetail(): Detail {
	const detail = detailFixture();
	const blocks = detail.speaker_blocks as Array<Record<string, unknown>>;
	blocks[1].label = 'Alice';
	return detail;
}

function stubInvoke(overrides: Record<string, unknown | ((args: unknown) => unknown)> = {}) {
	mockedInvoke.mockImplementation(async (cmd: string, args?: unknown) => {
		if (cmd in overrides) {
			const value = overrides[cmd];
			return typeof value === 'function' ? (value as (a: unknown) => unknown)(args) : value;
		}
		switch (cmd) {
			case 'note_render_transcript_html':
				return '';
			case 'history_get_detail':
				return detailFixture();
			case 'settings_get_input_labels':
				return ['Mic', 'Speaker'];
			case 'speaker_names_list':
				return [{ slug: 'alice', name: 'Alice' }];
			case 'settings_get_user_display_name':
				return 'You';
			default:
				return undefined;
		}
	});
}

async function renderPanel() {
	const result = render(TranscriptPanel, { props: { noteId: 'note-1' } });
	await waitFor(() => {
		expect(screen.getByText('first block')).toBeInTheDocument();
	});
	return result;
}

describe('TranscriptPanel speaker relabeling', () => {
	beforeEach(() => {
		mockedInvoke.mockReset();
		stubInvoke();
	});

	it('opens a picker with saved names, other block labels, and Other', async () => {
		await renderPanel();

		await fireEvent.click(screen.getByRole('button', { name: '[Speaker 2]' }));

		expect(screen.getByText('Who is speaking?')).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Alice' })).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Speaker 1' })).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Other' })).toBeInTheDocument();
		// The block's current label is not offered as a target.
		expect(screen.queryByRole('button', { name: 'Speaker 2' })).not.toBeInTheDocument();
	});

	it('relabels via note_relabel_speaker and re-renders from the returned record', async () => {
		stubInvoke({ note_relabel_speaker: relabeledDetail() });
		await renderPanel();

		await fireEvent.click(screen.getByRole('button', { name: '[Speaker 2]' }));
		await fireEvent.click(screen.getByRole('button', { name: 'Alice' }));

		await waitFor(() => {
			expect(mockedInvoke).toHaveBeenCalledWith('note_relabel_speaker', {
				id: 'note-1',
				fromLabel: 'Speaker 2',
				toLabel: 'Alice',
			});
		});
		await waitFor(() => {
			expect(screen.getByRole('button', { name: '[Alice]' })).toBeInTheDocument();
		});
	});

	it('relabels to a typed new name', async () => {
		stubInvoke({ note_relabel_speaker: relabeledDetail() });
		await renderPanel();

		await fireEvent.click(screen.getByRole('button', { name: '[Speaker 2]' }));
		const field = screen.getByLabelText('New speaker');
		await fireEvent.input(field, { target: { value: 'Ben' } });
		await fireEvent.click(screen.getByRole('button', { name: 'Save' }));

		await waitFor(() => {
			expect(mockedInvoke).toHaveBeenCalledWith('note_relabel_speaker', {
				id: 'note-1',
				fromLabel: 'Speaker 2',
				toLabel: 'Ben',
			});
		});
	});

	it('never calls voiceprint or learning IPC', async () => {
		stubInvoke({ note_relabel_speaker: relabeledDetail() });
		await renderPanel();

		await fireEvent.click(screen.getByRole('button', { name: '[Speaker 2]' }));
		await fireEvent.click(screen.getByRole('button', { name: 'Alice' }));
		await waitFor(() => {
			expect(screen.getByRole('button', { name: '[Alice]' })).toBeInTheDocument();
		});

		const commands = mockedInvoke.mock.calls.map(([cmd]) => cmd);
		expect(commands.some((cmd) => String(cmd).startsWith('voiceprint_'))).toBe(false);
	});

	it('does not offer relabeling on channel-tier (In/Out) blocks', async () => {
		stubInvoke({
			history_get_detail: {
				notes: [],
				speaker_blocks: [
					{ label: 'In', start_ms: 0, end_ms: 5_000, text: 'me talking' },
					{ label: 'Out', start_ms: 5_000, end_ms: 9_000, text: 'them talking' },
				],
			},
		});
		render(TranscriptPanel, { props: { noteId: 'note-1' } });
		await waitFor(() => {
			expect(screen.getByText('me talking')).toBeInTheDocument();
		});

		expect(screen.queryByRole('button', { name: '[Mic]' })).not.toBeInTheDocument();
		expect(screen.getByText('[Mic]')).toBeInTheDocument();
		expect(screen.getByText('[Speaker]')).toBeInTheDocument();
	});

	it('still shows correction badges on legacy chunk-based notes', async () => {
		stubInvoke({
			history_get_detail: {
				notes: [],
				speaker_blocks: [
					{ label: 'Alice', start_ms: 0, end_ms: 10_000, text: 'legacy block', chunk_id: 'chunk-0001' },
				],
				speaker_chunks: [
					{
						id: 'chunk-0001',
						label: 'Alice',
						corrections: [
							{ from_label: 'Speaker A', to_label: 'Alice', corrected_at_ms: 1, auto: false },
						],
					},
				],
			},
		});
		render(TranscriptPanel, { props: { noteId: 'note-1' } });
		await waitFor(() => {
			expect(screen.getByText('legacy block')).toBeInTheDocument();
		});

		expect(screen.getByText('corrected')).toBeInTheDocument();
	});
});
