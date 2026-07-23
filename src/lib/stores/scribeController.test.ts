import { tick } from 'svelte';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { appState } from './appState.svelte';
import { scribe } from './scribeController.svelte';
import { createCaptureEventEmitters, createEventBus } from '../../test/ipcFixtures';

const mockedInvoke = vi.mocked(invoke);
const mockedListen = vi.mocked(listen);
const mockedGetCurrentWindow = vi.mocked(getCurrentWindow);

describe('scribeController', () => {
	const bus = createEventBus();
	const events = createCaptureEventEmitters(bus);

	beforeEach(async () => {
		if (scribe.phase !== 'idle') {
			await scribe.discard();
		}
		scribe.destroy();
		appState.scribeNoteId = null;
		appState.scribeAwaitingAttach = false;
		mockedInvoke.mockReset();
		mockedListen.mockReset();
		bus.wireListen(mockedListen);
		mockedGetCurrentWindow.mockReturnValue({
			onFocusChanged: vi.fn().mockResolvedValue(() => {}),
		} as unknown as ReturnType<typeof getCurrentWindow>);

		mockedInvoke.mockImplementation(async (cmd: string) => {
			if (cmd === 'scribe_get_include_timestamps') return true;
			if (cmd === 'scribe_list_input_devices') return ['Built-in Microphone'];
			if (cmd === 'settings_get_preferred_audio_devices') return [null, null];
			if (cmd === 'settings_get_scribe_capture_speaker') return false;
		});

		await scribe.init();
	});

	it('starts recording and invokes scribe_start', async () => {
		await scribe.startRecording('note-abc');

		expect(mockedInvoke).toHaveBeenCalledWith('scribe_set_attach_note', { noteId: 'note-abc' });
		expect(mockedInvoke).toHaveBeenCalledWith('scribe_start', expect.objectContaining({
			captureSpeaker: false,
		}));
		expect(scribe.phase).toBe('recording');
		expect(appState.scribeNoteId).toBe('note-abc');
	});

	it('transitions through stop → transcribing → done via events', async () => {
		mockedInvoke.mockImplementation(async (cmd: string) => {
			if (cmd === 'scribe_stop_and_save') return undefined;
			if (cmd === 'note_attach_transcript') return undefined;
			if (cmd === 'scribe_get_include_timestamps') return true;
			if (cmd === 'scribe_list_input_devices') return [];
			if (cmd === 'settings_get_preferred_audio_devices') return [null, null];
			if (cmd === 'settings_get_scribe_capture_speaker') return false;
			return undefined;
		});

		await scribe.startRecording('note-abc');
		await scribe.stopAndSave();
		expect(scribe.phase).toBe('transcribing');
		expect(appState.scribeAwaitingAttach).toBe(true);

		events.scribeState({ state: 'DONE', progress: 1 });
		await tick();
		await tick();

		expect(scribe.phase).toBe('idle');
		expect(mockedInvoke).toHaveBeenCalledWith('note_attach_transcript', { id: 'note-abc' });
		expect(scribe.transcriptReadyNoteId).toBe('note-abc');
	});

	it('locks capture settings only while transcribing', async () => {
		expect(scribe.captureSettingsLocked).toBe(false);

		await scribe.startRecording('note-abc');
		expect(scribe.captureSettingsLocked).toBe(false);

		await scribe.stopAndSave();
		expect(scribe.captureSettingsLocked).toBe(true);
	});

	it('returns to idle on ERROR without auto-restart', async () => {
		await scribe.startRecording('note-abc');
		events.scribeState({ state: 'ERROR', error: 'Mic disconnected' });

		expect(scribe.phase).toBe('idle');
		expect(scribe.errorMessage).toBe('Mic disconnected');
		expect(appState.scribeNoteId).toBeNull();
	});

	it('does not leave idle on external IDLE event while not recording', () => {
		events.scribeState({ state: 'IDLE' });
		expect(scribe.phase).toBe('idle');
	});

	it('toggles speaker capture during recording', async () => {
		await scribe.startRecording('note-abc');
		await scribe.setSpeakerCapture(true);

		expect(mockedInvoke).toHaveBeenCalledWith('scribe_toggle_speaker_capture', { enabled: true });
		expect(scribe.captureSpeaker).toBe(true);
	});
});
