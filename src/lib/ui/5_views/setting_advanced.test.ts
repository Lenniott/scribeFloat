import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { invoke } from '@tauri-apps/api/core';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createInvokeRouter } from '../../../test/ipcFixtures';
import SettingAdvanced from './setting_advanced.svelte';

const mockedInvoke = vi.mocked(invoke);

describe('setting_advanced.svelte', () => {
	beforeEach(() => {
		mockedInvoke.mockReset();
		mockedInvoke.mockImplementation(
			createInvokeRouter({
				settings_get_save_transcripts_as_markdown: false,
				settings_get_keep_wav: false,
				settings_get_open_with_app_path: null,
				settings_get_voice_similarity_threshold: 0.75,
				settings_get_voice_embeddings_retention: 'keep',
			}),
		);
	});

	it('has no batch Save button', async () => {
		render(SettingAdvanced);

		await waitFor(() => {
			expect(mockedInvoke).toHaveBeenCalledWith('settings_get_keep_wav');
		});
		expect(screen.queryByRole('button', { name: 'Save' })).not.toBeInTheDocument();
	});

	it('persists markdown toggle immediately and reveals the open-with picker', async () => {
		render(SettingAdvanced);

		const toggle = await screen.findByRole('switch', { name: 'Save transcripts as Markdown' });
		await fireEvent.click(toggle);

		await waitFor(() => {
			expect(mockedInvoke).toHaveBeenCalledWith('settings_set_save_transcripts_as_markdown', {
				enabled: true,
			});
		});
		expect(screen.getAllByText('Open transcripts with').length).toBeGreaterThan(0);
	});

	it('shows a Saved toast after an auto-save', async () => {
		render(SettingAdvanced);

		const toggle = await screen.findByRole('switch', { name: 'Keep audio after transcription' });
		await fireEvent.click(toggle);

		await waitFor(() => {
			expect(screen.getByRole('status')).toHaveTextContent('Saved');
		});
	});

	it('debounces the sensitivity slider before saving', async () => {
		render(SettingAdvanced);
		const slider = await screen.findByRole('slider', {
			name: 'Speaker matching sensitivity',
		});

		vi.useFakeTimers();
		try {
			await fireEvent.input(slider, { target: { value: '0.9' } });
			expect(mockedInvoke).not.toHaveBeenCalledWith(
				'settings_set_voice_similarity_threshold',
				expect.anything(),
			);

			vi.advanceTimersByTime(350);
			expect(mockedInvoke).toHaveBeenCalledWith('settings_set_voice_similarity_threshold', {
				threshold: 0.9,
			});
		} finally {
			vi.useRealTimers();
		}
	});

	it('requires confirmation before removing voice vectors', async () => {
		render(SettingAdvanced);

		const removeButton = await screen.findByRole('button', { name: 'Remove vectors' });
		await fireEvent.click(removeButton);
		expect(mockedInvoke).not.toHaveBeenCalledWith('history_remove_all_voice_embeddings');

		await fireEvent.click(screen.getByRole('button', { name: 'Remove vectors' }));
		await waitFor(() => {
			expect(mockedInvoke).toHaveBeenCalledWith('history_remove_all_voice_embeddings');
		});
	});
});
