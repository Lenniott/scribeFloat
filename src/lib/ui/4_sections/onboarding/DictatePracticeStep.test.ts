import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { tick } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import DictatePracticeStep from './DictatePracticeStep.svelte';

type DictateState = 'IDLE' | 'RECORDING' | 'TRANSCRIBING' | 'PASTING' | 'DONE' | 'ERROR';
type DictateGesture = 'double_tap' | 'hold';
type DictateStateEvent = { state: DictateState; text?: string; error?: string; gesture?: DictateGesture };
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

	it('surfaces ERROR hint from dictate state event', async () => {
		renderStep();

		await waitFor(() => {
			expect(mockedListen).toHaveBeenCalledWith('dictate://state-changed', expect.any(Function));
		});

		stateChanged({ payload: { state: 'ERROR', error: 'No model installed' } });
		await tick();

		expect(screen.getByText('No model installed')).toBeInTheDocument();
	});

	it('persists auto-enter preference via settings_set_dictate_auto_enter', async () => {
		renderStep();

		await waitFor(() => {
			expect(screen.getByRole('switch', { name: 'Auto enter after dictate' })).toBeInTheDocument();
		});

		await fireEvent.click(screen.getByRole('switch', { name: 'Auto enter after dictate' }));
		await tick();

		expect(mockedInvoke).toHaveBeenCalledWith('settings_set_dictate_auto_enter', { enabled: true });
	});

	it('line-clamps long practice notes to two lines so Continue stays reachable', async () => {
		const { container } = renderStep();

		await waitFor(() => {
			expect(screen.getByPlaceholderText('Click here and test dictate')).toBeInTheDocument();
		});

		const long = `${'# Heading\n'.repeat(20)}and more body text `.repeat(10);
		const composer = screen.getByPlaceholderText('Click here and test dictate');
		await fireEvent.input(composer, { target: { value: long } });
		await fireEvent.keyDown(composer, { key: 'Enter' });
		await tick();

		const body = container.querySelector('article p.line-clamp-2');
		expect(body).toBeTruthy();
		expect(body).toHaveClass('line-clamp-2');
		const card = container.querySelector('article');
		expect(card).toHaveClass('h-auto');
		expect(card).toHaveClass('shrink-0');
		// Full text stays in the DOM; visual height is CSS-clamped.
		expect(body?.textContent?.length ?? 0).toBeGreaterThan(400);
		expect(screen.getByRole('button', { name: 'Continue' })).toBeEnabled();
	});

	it('credits a gesture as tried only once its RECORDING event reports it', async () => {
		renderStep();

		await waitFor(() => {
			expect(mockedListen).toHaveBeenCalledWith('dictate://state-changed', expect.any(Function));
		});

		const doubleTapRow = screen.getByText('Double-tap').closest('div');
		const holdRow = screen.getByText('Press-and-hold').closest('div');
		expect(doubleTapRow).toHaveClass('text-fg-muted');
		expect(holdRow).toHaveClass('text-fg-muted');

		stateChanged({ payload: { state: 'RECORDING', gesture: 'double_tap' } });
		await tick();

		expect(doubleTapRow).toHaveClass('text-success');
		expect(holdRow).toHaveClass('text-fg-muted');

		stateChanged({ payload: { state: 'RECORDING', gesture: 'hold' } });
		await tick();

		expect(holdRow).toHaveClass('text-success');
	});
});
