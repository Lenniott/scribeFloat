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

	it('has no voice matching or embedding controls', async () => {
		const { container } = render(SettingAdvanced);

		await waitFor(() => {
			expect(mockedInvoke).toHaveBeenCalledWith('settings_get_keep_wav');
		});
		expect(container.textContent).not.toMatch(/voice|embedding|sensitivity|vector/i);
		const commands = mockedInvoke.mock.calls.map(([cmd]) => cmd);
		expect(commands.some((cmd) => String(cmd).includes('voice'))).toBe(false);
	});
});
