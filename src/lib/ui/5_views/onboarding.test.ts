import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { tick } from 'svelte';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import Onboarding from './onboarding.svelte';

const mockedInvoke = vi.mocked(invoke);
const mockedGetCurrentWindow = vi.mocked(getCurrentWindow);

describe('onboarding orchestration', () => {
	const close = vi.fn();

	beforeEach(() => {
		mockedInvoke.mockReset();
		close.mockReset();
		mockedGetCurrentWindow.mockReturnValue({
			close,
			onFocusChanged: vi.fn().mockResolvedValue(() => {}),
		} as unknown as ReturnType<typeof getCurrentWindow>);

		mockedInvoke.mockImplementation(async () => undefined);
	});

	it('starts on Welcome step', async () => {
		render(Onboarding);

		expect(screen.getByRole('button', { name: 'Get started' })).toBeInTheDocument();
		expect(screen.queryByText(/Step 1 of/i)).not.toBeInTheDocument();
	});

	it('goes straight to Permissions after Welcome (no model step)', async () => {
		mockedInvoke.mockImplementation(async (cmd: string) => {
			if (cmd === 'settings_permissions_status') {
				return [{ kind: 'microphone', granted: true, can_request: false }];
			}
		});

		render(Onboarding);
		await tick();

		await fireEvent.click(screen.getByRole('button', { name: 'Get started' }));

		await waitFor(() => {
			expect(screen.getByText('Grant permissions')).toBeInTheDocument();
		});
		expect(screen.queryByText('Install AI model')).not.toBeInTheDocument();
	});

	it('has no voice enrollment step', async () => {
		render(Onboarding);

		const commands = mockedInvoke.mock.calls.map(([cmd]) => cmd);
		expect(commands.some((cmd) => String(cmd).startsWith('voiceprint_'))).toBe(false);
	});

	it('skip to settings completes onboarding and shows main window', async () => {
		render(Onboarding);

		await fireEvent.click(screen.getByRole('button', { name: 'Skip to Settings' }));

		await waitFor(() => {
			expect(mockedInvoke).toHaveBeenCalledWith('settings_complete_onboarding');
			expect(mockedInvoke).toHaveBeenCalledWith('settings_show_window');
			expect(close).toHaveBeenCalledOnce();
		});
	});
});
