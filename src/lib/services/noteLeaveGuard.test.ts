import { describe, expect, it, vi } from 'vitest';
import { runNoteLeaveGuard } from './noteLeaveGuard';
import type { NoteLeaveInvoke } from './noteLeaveGuard';

function handlers() {
	return {
		proceed: vi.fn(),
		cancel: vi.fn(),
		showMetadataDiscard: vi.fn(),
	};
}

describe('runNoteLeaveGuard', () => {
	it('proceeds immediately while recording', async () => {
		const h = handlers();
		await runNoteLeaveGuard(
			{
				id: 'n1',
				recordingActive: true,
				invoke: vi.fn(),
			},
			h,
		);
		expect(h.proceed).toHaveBeenCalledOnce();
		expect(h.showMetadataDiscard).not.toHaveBeenCalled();
	});

	it('deletes and proceeds for an empty note with no metadata', async () => {
		const h = handlers();
		const invoke = vi.fn(async (cmd: string) => {
			if (cmd === 'note_is_empty') return true;
			if (cmd === 'note_has_metadata') return false;
		}) as unknown as NoteLeaveInvoke;
		const onEmptyDeleted = vi.fn();

		await runNoteLeaveGuard(
			{ id: 'n1', recordingActive: false, invoke, onEmptyDeleted },
			h,
		);

		expect(invoke).toHaveBeenCalledWith('history_delete', { id: 'n1' });
		expect(onEmptyDeleted).toHaveBeenCalledOnce();
		expect(h.proceed).toHaveBeenCalledOnce();
	});

	it('shows discard modal for metadata-only empty notes', async () => {
		const h = handlers();
		const invoke = vi.fn(async (cmd: string) => {
			if (cmd === 'note_is_empty') return true;
			if (cmd === 'note_has_metadata') return true;
		}) as unknown as NoteLeaveInvoke;

		await runNoteLeaveGuard({ id: 'n1', recordingActive: false, invoke }, h);

		expect(h.showMetadataDiscard).toHaveBeenCalledOnce();
		expect(h.proceed).not.toHaveBeenCalled();
		expect(invoke).not.toHaveBeenCalledWith('history_delete', expect.anything());
	});

	it('proceeds without delete when note has content', async () => {
		const h = handlers();
		const invoke = vi.fn(async (cmd: string) => {
			if (cmd === 'note_is_empty') return false;
			if (cmd === 'note_has_metadata') return false;
		}) as unknown as NoteLeaveInvoke;

		await runNoteLeaveGuard({ id: 'n1', recordingActive: false, invoke }, h);

		expect(h.proceed).toHaveBeenCalledOnce();
		expect(invoke).not.toHaveBeenCalledWith('history_delete', expect.anything());
	});
});
