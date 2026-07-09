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
			{
				label: 'Speaker A',
				start_ms: 0,
				end_ms: 10_000,
				text: 'first block',
				chunk_id: 'chunk-0001',
			},
			{
				label: 'Speaker B',
				start_ms: 10_000,
				end_ms: 20_000,
				text: 'second block',
				chunk_id: 'chunk-0002',
			},
		],
		speaker_chunks: [
			{ id: 'chunk-0001', label: 'Speaker A', corrections: [] },
			{ id: 'chunk-0002', label: 'Speaker B', corrections: [] },
		],
		session_speakers: [
			{ session_speaker_id: 'speaker-1', label: 'Speaker A' },
			{ session_speaker_id: 'speaker-2', label: 'Speaker B' },
		],
	};
}

function correctedDetail(): Detail {
	const detail = detailFixture();
	const blocks = detail.speaker_blocks as Array<Record<string, unknown>>;
	blocks[1].label = 'Alice';
	const chunks = detail.speaker_chunks as Array<Record<string, unknown>>;
	chunks[1].label = 'Alice';
	chunks[1].corrections = [
		{ from_label: 'Speaker B', to_label: 'Alice', corrected_at_ms: 1, auto: false },
	];
	const speakers = detail.session_speakers as Array<Record<string, unknown>>;
	speakers[1].label = 'Alice';
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
			case 'voiceprint_list_profiles':
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

describe('TranscriptPanel speaker correction', () => {
	beforeEach(() => {
		mockedInvoke.mockReset();
		stubInvoke();
	});

	it('opens a picker with profiles, other session speakers, and Other', async () => {
		await renderPanel();

		await fireEvent.click(screen.getByRole('button', { name: '[Speaker B]' }));

		expect(screen.getByText('Who is speaking?')).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Alice' })).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Speaker A' })).toBeInTheDocument();
		expect(screen.getByRole('button', { name: 'Other' })).toBeInTheDocument();
		// The block's current label is not offered as a correction target.
		expect(screen.queryByRole('button', { name: 'Speaker B' })).not.toBeInTheDocument();
	});

	it('applies a correction and re-renders labels from the updated record', async () => {
		stubInvoke({ note_correct_chunk_label: correctedDetail() });
		await renderPanel();

		await fireEvent.click(screen.getByRole('button', { name: '[Speaker B]' }));
		await fireEvent.click(screen.getByRole('button', { name: 'Alice' }));

		await waitFor(() => {
			expect(mockedInvoke).toHaveBeenCalledWith('note_correct_chunk_label', {
				id: 'note-1',
				chunkId: 'chunk-0002',
				label: 'Alice',
			});
		});
		await waitFor(() => {
			expect(screen.getByRole('button', { name: '[Alice]' })).toBeInTheDocument();
		});
		expect(screen.getByText('corrected')).toBeInTheDocument();
	});

	it('marks cascade relabels as auto-corrected', async () => {
		const detail = correctedDetail();
		const chunks = detail.speaker_chunks as Array<Record<string, unknown>>;
		chunks[0].corrections = [
			{ from_label: 'Speaker A', to_label: 'Alice', corrected_at_ms: 2, auto: true },
		];
		stubInvoke({ note_correct_chunk_label: detail });
		await renderPanel();

		await fireEvent.click(screen.getByRole('button', { name: '[Speaker B]' }));
		await fireEvent.click(screen.getByRole('button', { name: 'Alice' }));

		await waitFor(() => {
			expect(screen.getByText('auto-corrected')).toBeInTheDocument();
		});
	});

	it('offers to improve the voiceprint when evidence passes the gates', async () => {
		stubInvoke({
			note_correct_chunk_label: correctedDetail(),
			voiceprint_evaluate_session_evidence: { eligible: true, reasons: [] },
		});
		await renderPanel();

		await fireEvent.click(screen.getByRole('button', { name: '[Speaker B]' }));
		await fireEvent.click(screen.getByRole('button', { name: 'Alice' }));

		await waitFor(() => {
			expect(
				screen.getByText("Improve Alice's voiceprint from this recording?"),
			).toBeInTheDocument();
		});

		await fireEvent.click(screen.getByRole('button', { name: 'Add' }));
		await waitFor(() => {
			expect(mockedInvoke).toHaveBeenCalledWith('voiceprint_apply_session_evidence', {
				noteId: 'note-1',
				sessionSpeakerId: 'speaker-2',
				profileName: 'Alice',
			});
		});
	});

	it('stays quiet when the learning evaluation is rejected or unavailable', async () => {
		stubInvoke({
			note_correct_chunk_label: correctedDetail(),
			voiceprint_evaluate_session_evidence: () => {
				throw new Error('voice learning is disabled in settings');
			},
		});
		await renderPanel();

		await fireEvent.click(screen.getByRole('button', { name: '[Speaker B]' }));
		await fireEvent.click(screen.getByRole('button', { name: 'Alice' }));

		await waitFor(() => {
			expect(screen.getByRole('button', { name: '[Alice]' })).toBeInTheDocument();
		});
		expect(screen.queryByText(/Improve/)).not.toBeInTheDocument();
	});
});
