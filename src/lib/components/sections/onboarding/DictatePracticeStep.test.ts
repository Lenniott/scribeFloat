import { render, screen, waitFor } from '@testing-library/svelte';
import { tick } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import DictatePracticeStep from './DictatePracticeStep.svelte';

type DictateState = 'IDLE' | 'RECORDING' | 'TRANSCRIBING' | 'PASTING' | 'DONE' | 'ERROR';
type DictateStateEvent = { state: DictateState; text?: string; error?: string };
type EventCallback = (event: { payload: DictateStateEvent }) => void;

const mockedInvoke = vi.mocked(invoke);
const mockedListen = vi.mocked(listen);

function renderStep() {
	return render(DictatePracticeStep, {
		props: {
			onBack: vi.fn(),
			onNext: vi.fn(),
		},
	});
}

describe('DictatePracticeStep', () => {
	let stateChanged: EventCallback;

	beforeEach(() => {
		mockedInvoke.mockReset();
		mockedListen.mockReset();
		mockedInvoke.mockResolvedValue(false);
		mockedListen.mockImplementation(async (_event, callback) => {
			stateChanged = callback as EventCallback;
			return vi.fn();
		});
	});

	it('loads Auto Enter as off when no preference is enabled', async () => {
		renderStep();

		await waitFor(() => {
			expect(mockedInvoke).toHaveBeenCalledWith('settings_get_dictate_auto_enter');
		});

		expect(screen.getByRole('switch', { name: 'Auto enter after dictate' })).not.toBeChecked();
	});

	it('does not copy DONE text into the practice textarea', async () => {
		renderStep();

		await waitFor(() => {
			expect(mockedListen).toHaveBeenCalledWith('dictate://state-changed', expect.any(Function));
		});

		stateChanged({ payload: { state: 'DONE', text: 'hello from dictate' } });
		await tick();

		expect(screen.getByPlaceholderText('Click here and test dictate')).toHaveValue('');
		expect(screen.queryByText('hello from dictate')).not.toBeInTheDocument();
	});

	it('keeps the practice textarea visible during Dictate processing', async () => {
		renderStep();

		await waitFor(() => {
			expect(mockedListen).toHaveBeenCalledWith('dictate://state-changed', expect.any(Function));
		});

		stateChanged({ payload: { state: 'TRANSCRIBING' } });
		await tick();

		expect(screen.getByPlaceholderText('Click here and test dictate')).toBeVisible();
		expect(screen.queryByText('Transcribing…')).not.toBeInTheDocument();
	});
});
