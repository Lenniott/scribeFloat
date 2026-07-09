import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { tick } from 'svelte';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createCaptureEventEmitters, createEventBus } from '../../../test/ipcFixtures';
import DictateView from './dictate.svelte';

const mockedInvoke = vi.mocked(invoke);
const mockedListen = vi.mocked(listen);

describe('dictate.svelte', () => {
	const bus = createEventBus();
	const events = createCaptureEventEmitters(bus);

	beforeEach(() => {
		mockedInvoke.mockReset();
		mockedListen.mockReset();
		bus.wireListen(mockedListen);
		mockedInvoke.mockResolvedValue(undefined);
	});

	afterEach(() => {
		events.dictateState({ state: 'IDLE' });
	});

	it('shows recording HUD while RECORDING', async () => {
		render(DictateView);

		await waitFor(() => {
			expect(mockedListen).toHaveBeenCalledWith('dictate://state-changed', expect.any(Function));
		});

		events.dictateState({ state: 'RECORDING' });
		await tick();

		expect(screen.getByRole('button', { name: 'Cancel dictation' })).toBeInTheDocument();
	});

	it('shows progress while TRANSCRIBING', async () => {
		render(DictateView);

		await waitFor(() => {
			expect(mockedListen).toHaveBeenCalledWith('dictate://state-changed', expect.any(Function));
		});

		events.dictateState({ state: 'RECORDING' });
		events.dictateState({
			state: 'TRANSCRIBING',
			processing_stage: 'TRANSCRIBING_AUDIO',
			progress: 0.4,
		});
		await tick();

		expect(screen.getByRole('progressbar')).toBeInTheDocument();
	});

	it('shows DONE text and history write failure warning', async () => {
		render(DictateView);

		await waitFor(() => {
			expect(mockedListen).toHaveBeenCalledWith('dictate://state-changed', expect.any(Function));
		});

		events.dictateState({
			state: 'DONE',
			text: 'Hello world',
			history_write_failed: true,
		});
		await tick();

		expect(screen.getByText('Hello world')).toBeInTheDocument();
		expect(
			screen.getByText('History entry could not be saved — check save folder.'),
		).toBeInTheDocument();
	});

	it('shows ERROR message', async () => {
		render(DictateView);

		await waitFor(() => {
			expect(mockedListen).toHaveBeenCalledWith('dictate://state-changed', expect.any(Function));
		});

		events.dictateState({ state: 'ERROR', error: 'Transcription failed' });
		await tick();

		expect(screen.getByText('Transcription failed')).toBeInTheDocument();
	});

	it('cancels dictation when close is clicked during TRANSCRIBING', async () => {
		render(DictateView);

		await waitFor(() => {
			expect(mockedListen).toHaveBeenCalledWith('dictate://state-changed', expect.any(Function));
		});

		events.dictateState({ state: 'TRANSCRIBING' });
		await tick();

		await fireEvent.click(screen.getByRole('button', { name: 'Cancel dictation' }));

		expect(mockedInvoke).toHaveBeenCalledWith('dictate_cancel');
	});
});
