import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { appState } from './appState.svelte';

export type ScribePhase = 'idle' | 'recording' | 'transcribing';

export type ScribeProcessingStage =
	| 'LOADING_MODEL'
	| 'TRANSCRIBING_AUDIO'
	| 'WRITING_TRANSCRIPT'
	| 'CLEANING_UP_AUDIO';

type ScribePayload = {
	state: string;
	error?: string;
	progress?: number;
	processing_stage?: ScribeProcessingStage;
};

class ScribeController {
	phase = $state<ScribePhase>('idle');
	elapsedMs = $state(0);
	audioLevel = $state(0);
	speakerLevel = $state(0);
	selectedMic = $state('');
	captureSpeaker = $state(false);
	includeTimestamps = $state(true);
	micOptions = $state([{ value: '', label: 'System Default' }]);
	errorMessage = $state('');
	progress = $state(0);
	processingStage = $state<ScribeProcessingStage>('LOADING_MODEL');
	/** Set when a transcript was attached; note editor clears after handling. */
	transcriptReadyNoteId = $state<string | null>(null);

	private timerInterval: ReturnType<typeof setInterval> | null = null;
	private awaitingAttach = false;
	private unlisteners: UnlistenFn[] = [];
	private initialized = false;

	elapsedSeconds = $derived(Math.floor(this.elapsedMs / 1000));
	progressPercent = $derived(Math.round(Math.max(0, Math.min(1, this.progress)) * 100));
	/** Mic/speaker controls are unavailable only while transcribing. */
	captureSettingsLocked = $derived(this.phase === 'transcribing');

	async init() {
		if (this.initialized) return;
		this.initialized = true;
		await this.loadSettings();
		this.unlisteners.push(
			await listen<ScribePayload>('scribe://state-changed', (e) =>
				this.handleScribeEvent(e.payload),
			),
			await listen<number>('scribe://audio-level', (e) => {
				this.audioLevel = e.payload;
			}),
			await listen<number>('scribe://speaker-level', (e) => {
				this.speakerLevel = e.payload;
			}),
		);
	}

	destroy() {
		for (const ul of this.unlisteners) ul();
		this.unlisteners = [];
		this.stopTimer();
		this.initialized = false;
	}

	clearTranscriptReady() {
		this.transcriptReadyNoteId = null;
	}

	isRecordingToNote(noteId: string): boolean {
		return this.phase === 'recording' && appState.scribeNoteId === noteId;
	}

	async loadSettings() {
		this.includeTimestamps = await invoke<boolean>('scribe_get_include_timestamps').catch(() => true);
		const devices = await invoke<string[]>('scribe_list_input_devices').catch(() => []);
		this.micOptions = [
			{ value: '', label: 'System Default' },
			...devices.map((d) => ({ value: d, label: d })),
		];
		const [preferredMic] = await invoke<[string | null, string | null]>(
			'settings_get_preferred_audio_devices',
		).catch(() => [null, null] as [string | null, string | null]);
		this.selectedMic = preferredMic ?? '';
		this.captureSpeaker = await invoke<boolean>('settings_get_scribe_capture_speaker').catch(
			() => false,
		);
	}

	async setMic(mic: string) {
		const [, preferredSpeaker] = await invoke<[string | null, string | null]>(
			'settings_get_preferred_audio_devices',
		).catch(() => [null, null] as [string | null, string | null]);

		await invoke('settings_set_preferred_audio_devices', {
			preferredInputDevice: mic || null,
			preferredSpeakerDevice: preferredSpeaker,
		}).catch(() => {});

		this.selectedMic = mic;

		if (this.phase !== 'recording') return;

		const noteId = appState.scribeNoteId;
		if (!noteId) return;

		// Same as legacy Scribe screen: cancel and restart on the same note with the new mic.
		const sessionSpeakerCapture = this.captureSpeaker;
		await invoke('scribe_set_attach_note', { noteId: null }).catch(() => {});
		await invoke('scribe_cancel').catch(() => {});
		this.stopTimer();
		this.audioLevel = 0;
		this.speakerLevel = 0;
		this.phase = 'idle';
		appState.scribeNoteId = null;
		this.captureSpeaker = sessionSpeakerCapture;
		await this.startRecording(noteId);
	}

	/** Idle: persist default. Recording: session-only toggle via backend. */
	async setSpeakerCapture(enabled: boolean) {
		if (this.phase === 'transcribing') return;

		if (this.phase === 'recording') {
			try {
				await invoke('scribe_toggle_speaker_capture', { enabled });
				this.captureSpeaker = enabled;
			} catch (e) {
				this.errorMessage = String(e);
				throw e;
			}
			return;
		}

		try {
			await invoke('settings_set_scribe_capture_speaker', { enabled });
			this.captureSpeaker = enabled;
		} catch (e) {
			this.errorMessage = String(e);
			throw e;
		}
	}

	async startRecording(noteId: string) {
		if (this.phase !== 'idle') return;
		this.errorMessage = '';
		await invoke('scribe_set_attach_note', { noteId });
		await invoke('scribe_start', {
			preferredMic: this.selectedMic || null,
			preferredSpeaker: null,
			captureSpeaker: this.captureSpeaker,
		});
		appState.scribeNoteId = noteId;
		this.phase = 'recording';
		this.startTimer();
	}

	async stopAndSave() {
		if (this.phase !== 'recording') return;
		this.phase = 'transcribing';
		this.progress = 0;
		this.processingStage = 'LOADING_MODEL';
		this.stopTimer();
		this.audioLevel = 0;
		this.speakerLevel = 0;
		this.awaitingAttach = true;
		appState.scribeAwaitingAttach = true;
		try {
			await invoke('scribe_stop_and_save', { title: null });
		} catch (e) {
			this.awaitingAttach = false;
			appState.scribeAwaitingAttach = false;
			this.phase = 'idle';
			this.errorMessage = String(e);
		}
	}

	async discard() {
		this.stopTimer();
		this.audioLevel = 0;
		this.speakerLevel = 0;
		this.awaitingAttach = false;
		appState.scribeNoteId = null;
		appState.scribeAwaitingAttach = false;
		await invoke('scribe_set_attach_note', { noteId: null }).catch(() => {});
		await invoke('scribe_cancel').catch(() => {});
		this.phase = 'idle';
		void this.reloadSpeakerDefault();
	}

	private async reloadSpeakerDefault() {
		this.captureSpeaker = await invoke<boolean>('settings_get_scribe_capture_speaker').catch(
			() => false,
		);
	}

	private startTimer() {
		this.stopTimer();
		const start = Date.now();
		this.elapsedMs = 0;
		this.timerInterval = setInterval(() => {
			this.elapsedMs = Date.now() - start;
		}, 100);
	}

	private stopTimer() {
		if (this.timerInterval) {
			clearInterval(this.timerInterval);
			this.timerInterval = null;
		}
	}

	private async handleDone() {
		if (!this.awaitingAttach) return;
		const noteId = appState.scribeNoteId;
		this.awaitingAttach = false;
		appState.scribeAwaitingAttach = false;
		appState.scribeNoteId = null;
		this.phase = 'idle';
		void this.reloadSpeakerDefault();
		if (!noteId) return;
		try {
			await invoke('note_attach_transcript', { id: noteId });
			this.transcriptReadyNoteId = noteId;
		} catch (e) {
			this.errorMessage = String(e);
		}
	}

	private handleScribeEvent(p: ScribePayload) {
		if (p.progress != null) {
			this.progress = p.progress;
		}
		if (p.processing_stage) {
			this.processingStage = p.processing_stage;
		}

		switch (p.state) {
			case 'IDLE':
				if (!this.awaitingAttach) {
					this.phase = 'idle';
					this.progress = 0;
					this.processingStage = 'LOADING_MODEL';
					this.stopTimer();
					this.audioLevel = 0;
					this.speakerLevel = 0;
					appState.scribeNoteId = null;
					void this.reloadSpeakerDefault();
				}
				break;
			case 'RECORDING':
				this.phase = 'recording';
				if (!this.timerInterval) this.startTimer();
				break;
			case 'TRANSCRIBING':
				this.phase = 'transcribing';
				this.stopTimer();
				this.audioLevel = 0;
				this.speakerLevel = 0;
				break;
			case 'DONE':
				this.progress = 1;
				void this.handleDone();
				break;
			case 'ERROR':
				this.awaitingAttach = false;
				appState.scribeNoteId = null;
				appState.scribeAwaitingAttach = false;
				this.phase = 'idle';
				this.stopTimer();
				this.errorMessage = p.error ?? 'Recording failed';
				void this.reloadSpeakerDefault();
				break;
		}
	}
}

export const scribe = new ScribeController();
