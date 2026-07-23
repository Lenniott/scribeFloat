import type { invoke as tauriInvoke } from '@tauri-apps/api/core';

export type NoteLeaveInvoke = typeof tauriInvoke;

export type NoteLeaveGuardContext = {
	id: string;
	recordingActive: boolean;
	invoke: NoteLeaveInvoke;
	onEmptyDeleted?: () => void | Promise<void>;
};

export type NoteLeaveGuardHandlers = {
	proceed: () => void;
	cancel: () => void;
	showMetadataDiscard: () => void;
};

/**
 * Decide whether to delete, prompt, or keep a note when navigating away from the editor.
 * Metadata-only notes (tags/keywords/layers, no body or transcript) show a discard prompt.
 */
export async function runNoteLeaveGuard(
	ctx: NoteLeaveGuardContext,
	handlers: NoteLeaveGuardHandlers,
): Promise<void> {
	if (ctx.recordingActive) {
		handlers.proceed();
		return;
	}

	const [empty, hasMeta] = await Promise.all([
		ctx.invoke<boolean>('note_is_empty', { id: ctx.id }),
		ctx.invoke<boolean>('note_has_metadata', { id: ctx.id }),
	]);

	if (empty && hasMeta) {
		handlers.showMetadataDiscard();
		return;
	}

	if (empty) {
		await ctx.invoke('history_delete', { id: ctx.id });
		await ctx.onEmptyDeleted?.();
		handlers.proceed();
		return;
	}

	handlers.proceed();
}
