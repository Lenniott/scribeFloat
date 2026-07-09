import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { invoke } from '@tauri-apps/api/core';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { makePermissionStatuses } from '../../../../test/ipcFixtures';
import PermissionsStep from './PermissionsStep.svelte';

const mockedInvoke = vi.mocked(invoke);

describe('PermissionsStep', () => {
	beforeEach(() => {
		mockedInvoke.mockReset();
		mockedInvoke.mockImplementation(async (cmd: string) => {
			if (cmd === 'settings_permissions_status') {
				return makePermissionStatuses(false);
			}
		});
	});

	it('disables Continue until microphone is granted', async () => {
		render(PermissionsStep, { props: { onBack: vi.fn(), onNext: vi.fn() } });

		await waitFor(() => {
			expect(screen.getByRole('button', { name: 'Continue' })).toBeDisabled();
		});
	});

	it('enables Continue when microphone is granted', async () => {
		mockedInvoke.mockImplementation(async (cmd: string) => {
			if (cmd === 'settings_permissions_status') {
				return makePermissionStatuses(true);
			}
		});

		render(PermissionsStep, { props: { onBack: vi.fn(), onNext: vi.fn() } });

		await waitFor(() => {
			expect(screen.getByRole('button', { name: 'Continue' })).toBeEnabled();
		});
	});

	it('calls onNext when Continue is clicked with mic granted', async () => {
		const onNext = vi.fn();
		mockedInvoke.mockImplementation(async (cmd: string) => {
			if (cmd === 'settings_permissions_status') {
				return makePermissionStatuses(true);
			}
		});

		render(PermissionsStep, { props: { onBack: vi.fn(), onNext } });

		await waitFor(() => {
			expect(screen.getByRole('button', { name: 'Continue' })).toBeEnabled();
		});

		await fireEvent.click(screen.getByRole('button', { name: 'Continue' }));
		expect(onNext).toHaveBeenCalledOnce();
	});
});
