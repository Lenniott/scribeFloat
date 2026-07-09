import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { invoke } from '@tauri-apps/api/core';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createInvokeRouter } from '../../../test/ipcFixtures';
import SettingGeneral from './setting_general.svelte';

const mockedInvoke = vi.mocked(invoke);

describe('setting_general.svelte', () => {
	beforeEach(() => {
		mockedInvoke.mockReset();
		mockedInvoke.mockImplementation(
			createInvokeRouter({
				settings_get_output_path: '/tmp/transcripts',
				settings_get_preferred_audio_devices: [null, null],
				settings_get_scribe_capture_speaker: false,
				settings_get_dictate_auto_enter: false,
			}),
		);
	});

	it('has no batch Save button', async () => {
		render(SettingGeneral);

		await waitFor(() => {
			expect(mockedInvoke).toHaveBeenCalledWith('settings_get_output_path');
		});
		expect(screen.queryByRole('button', { name: 'Save' })).not.toBeInTheDocument();
	});

	it('persists the dictate auto-enter toggle immediately on change', async () => {
		render(SettingGeneral);

		const toggle = await screen.findByRole('switch', { name: 'Press Enter after dictate' });
		await fireEvent.click(toggle);

		await waitFor(() => {
			expect(mockedInvoke).toHaveBeenCalledWith('settings_set_dictate_auto_enter', {
				enabled: true,
			});
		});
	});

	it('shows a Saved toast after an auto-save', async () => {
		render(SettingGeneral);

		const toggle = await screen.findByRole('switch', { name: 'Press Enter after dictate' });
		await fireEvent.click(toggle);

		await waitFor(() => {
			expect(screen.getByRole('status')).toHaveTextContent('Saved');
		});
	});

	it('does not show a Saved toast when the save fails', async () => {
		mockedInvoke.mockImplementation(async (cmd: string) => {
			if (cmd === 'settings_set_dictate_auto_enter') throw new Error('disk full');
			return createInvokeRouter({
				settings_get_output_path: '/tmp/transcripts',
				settings_get_preferred_audio_devices: [null, null],
				settings_get_scribe_capture_speaker: false,
				settings_get_dictate_auto_enter: false,
			})(cmd);
		});
		render(SettingGeneral);

		const toggle = await screen.findByRole('switch', { name: 'Press Enter after dictate' });
		await fireEvent.click(toggle);

		await waitFor(() => {
			expect(screen.getByText(/Could not save dictate setting/)).toBeInTheDocument();
		});
		expect(screen.queryByRole('status')).not.toBeInTheDocument();
	});

	it('persists capture-speaker default immediately on change', async () => {
		render(SettingGeneral);

		const toggle = await screen.findByRole('switch', { name: 'Capture speaker by default' });
		await fireEvent.click(toggle);

		await waitFor(() => {
			expect(mockedInvoke).toHaveBeenCalledWith('settings_set_scribe_capture_speaker', {
				enabled: true,
			});
		});
	});
});
