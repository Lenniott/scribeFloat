import type { HistoryListItem, TagVocabularyEntry } from '@services/historyActions';
import type { PermissionStatus } from '@utils/types';
import type { Mock } from 'vitest';

export type EventCallback<T = unknown> = (event: { payload: T }) => void;

/** Build a `HistoryListItem` with sensible defaults. */
export function makeHistoryItem(overrides: Partial<HistoryListItem> = {}): HistoryListItem {
	return {
		id: 'note-1',
		kind: 'scribe',
		created_at: '2026-01-15T10:00:00.000Z',
		title: 'Test note',
		model: 'base',
		word_count: 100,
		duration_ms: 60_000,
		duration_secs: 60,
		excerpt: 'Sample excerpt text.',
		tags: ['work'],
		has_markdown: false,
		source: 'store',
		...overrides,
	};
}

export const historyFixtures = {
	storeScribe: () =>
		makeHistoryItem({ id: 'store-scribe-1', kind: 'scribe', source: 'store' }),
	storeDictate: () =>
		makeHistoryItem({ id: 'store-dictate-1', kind: 'dictate', source: 'store' }),
	transcribeUpload: () =>
		makeHistoryItem({ id: 'upload-1', kind: 'transcribe', source: 'store', title: 'interview.wav' }),
	writtenNote: () =>
		makeHistoryItem({ id: 'written-1', kind: 'written', source: 'store', title: 'Meeting notes' }),
	legacyMarkdown: () =>
		makeHistoryItem({
			id: 'md::legacy-1',
			kind: 'scribe',
			source: 'legacy',
			has_markdown: true,
			markdown_path: '/tmp/legacy.md',
		}),
	legacyDictate: () =>
		makeHistoryItem({
			id: 'dictate::legacy-1',
			kind: 'dictate',
			source: 'legacy',
		}),
	mixedList: (): HistoryListItem[] => [
		historyFixtures.storeScribe(),
		historyFixtures.storeDictate(),
		historyFixtures.transcribeUpload(),
		historyFixtures.writtenNote(),
		historyFixtures.legacyMarkdown(),
	],
};

export function makeTagVocabulary(): TagVocabularyEntry[] {
	return [
		{ name: 'work', count: 3 },
		{ name: 'meeting', count: 2 },
		{ name: 'personal', count: 1 },
	];
}

export function makePermissionStatuses(micGranted = false): PermissionStatus[] {
	return [
		{ kind: 'microphone', granted: micGranted, can_request: !micGranted },
		{ kind: 'accessibility', granted: false, can_request: true },
		{ kind: 'input_monitoring', granted: false, can_request: true },
	];
}

/** In-memory event bus for simulating Tauri `listen` / backend emits. */
export function createEventBus() {
	const listeners = new Map<string, Set<EventCallback>>();

	function on<T>(event: string, callback: EventCallback<T>): () => void {
		const set = listeners.get(event) ?? new Set();
		set.add(callback as EventCallback);
		listeners.set(event, set);
		return () => set.delete(callback as EventCallback);
	}

	function emit<T>(event: string, payload: T) {
		for (const cb of listeners.get(event) ?? []) {
			cb({ payload });
		}
	}

	function wireListen(mockedListen: Mock) {
		mockedListen.mockImplementation(async (event: string, callback: EventCallback) => {
			return on(event, callback);
		});
	}

	return { on, emit, wireListen };
}

export type ScribeStatePayload = {
	state: string;
	error?: string;
	progress?: number;
	processing_stage?: string;
};

export type DictateStatePayload = {
	state: 'IDLE' | 'RECORDING' | 'TRANSCRIBING' | 'PASTING' | 'DONE' | 'ERROR';
	progress?: number;
	processing_stage?: string;
	text?: string;
	paste_failed?: boolean;
	history_write_failed?: boolean;
	error?: string;
};

export type TranscribeStatePayload = {
	state: string;
	progress?: number;
	transcript_path?: string;
	error?: string;
};

/** Typed emit helpers for capture workflows. */
export function createCaptureEventEmitters(bus: ReturnType<typeof createEventBus>) {
	return {
		scribeState: (payload: ScribeStatePayload) => bus.emit('scribe://state-changed', payload),
		scribeAudioLevel: (level: number) => bus.emit('scribe://audio-level', level),
		scribeSpeakerLevel: (level: number) => bus.emit('scribe://speaker-level', level),
		dictateState: (payload: DictateStatePayload) => bus.emit('dictate://state-changed', payload),
		dictateAudioLevel: (level: number) => bus.emit('dictate://audio-level', level),
		transcribeState: (payload: TranscribeStatePayload) =>
			bus.emit('transcribe://state-changed', payload),
		transcribeItemProgress: (payload: { id: string; progress: number }) =>
			bus.emit('transcribe://item-progress', payload),
	};
}

type InvokeHandler = (args?: Record<string, unknown>) => unknown;

/** Build a `mockImplementation` handler map for `invoke`. */
export function createInvokeRouter(
	handlers: Record<string, unknown | InvokeHandler> = {},
) {
	// `args` is typed loosely so the router matches `invoke`'s InvokeArgs parameter.
	return async (cmd: string, args?: unknown) => {
		const handler = handlers[cmd];
		if (handler === undefined) return undefined;
		if (typeof handler === 'function') {
			return (handler as InvokeHandler)(args as Record<string, unknown> | undefined);
		}
		return handler;
	};
}
