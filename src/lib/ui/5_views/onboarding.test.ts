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

		mockedInvoke.mockImplementation(async (cmd: string) => {
			if (cmd === 'settings_get_onboarding_step') return 1;
			return undefined;
		});
	});

	it('starts on Welcome step', async () => {
		render(Onboarding);

		await waitFor(() => {
			expect(screen.getByRole('button', { name: 'Get started' })).toBeInTheDocument();
		});
		expect(screen.queryByText(/Step 1 of/i)).not.toBeInTheDocument();
	});

	it('restores saved Permissions step on mount', async () => {
		mockedInvoke.mockImplementation(async (cmd: string) => {
			if (cmd === 'settings_get_onboarding_step') return 2;
			if (cmd === 'settings_permissions_status') {
				return [{ kind: 'microphone', granted: true, can_request: false }];
			}
			return undefined;
		});

		render(Onboarding);

		await waitFor(() => {
			expect(screen.getByText('Grant permissions')).toBeInTheDocument();
		});
		expect(screen.queryByRole('button', { name: 'Get started' })).not.toBeInTheDocument();
	});

	it('persists step when leaving Welcome', async () => {
		mockedInvoke.mockImplementation(async (cmd: string) => {
			if (cmd === 'settings_get_onboarding_step') return 1;
			if (cmd === 'settings_permissions_status') {
				return [{ kind: 'microphone', granted: true, can_request: false }];
			}
			return undefined;
		});

		render(Onboarding);
		await waitFor(() => {
			expect(screen.getByRole('button', { name: 'Get started' })).toBeInTheDocument();
		});

		await fireEvent.click(screen.getByRole('button', { name: 'Get started' }));

		await waitFor(() => {
			expect(mockedInvoke).toHaveBeenCalledWith('settings_set_onboarding_step', { step: 2 });
			expect(screen.getByText('Grant permissions')).toBeInTheDocument();
		});
	});

	it('goes straight to Permissions after Welcome (no model step)', async () => {
		mockedInvoke.mockImplementation(async (cmd: string) => {
			if (cmd === 'settings_get_onboarding_step') return 1;
			if (cmd === 'settings_permissions_status') {
				return [{ kind: 'microphone', granted: true, can_request: false }];
			}
			return undefined;
		});

		render(Onboarding);
		await tick();

		await waitFor(() => {
			expect(screen.getByRole('button', { name: 'Get started' })).toBeInTheDocument();
		});
		await fireEvent.click(screen.getByRole('button', { name: 'Get started' }));

		await waitFor(() => {
			expect(screen.getByText('Grant permissions')).toBeInTheDocument();
		});
		expect(screen.queryByText('Install AI model')).not.toBeInTheDocument();
	});

	it('has no voice enrollment step', async () => {
		render(Onboarding);

		await waitFor(() => {
			expect(mockedInvoke).toHaveBeenCalledWith('settings_get_onboarding_step');
		});

		const commands = mockedInvoke.mock.calls.map(([cmd]) => cmd);
		expect(commands.some((cmd) => String(cmd).startsWith('voiceprint_'))).toBe(false);
	});

	it('skip to settings completes onboarding and shows main window', async () => {
		render(Onboarding);

		await waitFor(() => {
			expect(screen.getByRole('button', { name: 'Skip to Settings' })).toBeInTheDocument();
		});
		await fireEvent.click(screen.getByRole('button', { name: 'Skip to Settings' }));

		await waitFor(() => {
			expect(mockedInvoke).toHaveBeenCalledWith('settings_complete_onboarding');
			expect(mockedInvoke).toHaveBeenCalledWith('settings_show_window');
			expect(close).toHaveBeenCalledOnce();
		});
	});
});
