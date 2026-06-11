import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { createModelDownloadStore } from './modelDownload.svelte';

const mockedInvoke = vi.mocked(invoke);

describe('createModelDownloadStore', () => {
    beforeEach(() => {
        mockedInvoke.mockReset();
        mockedInvoke.mockResolvedValue(undefined);
    });

    it('exposes an empty model list initially', () => {
        const store = createModelDownloadStore();
        expect(store.models).toEqual([]);
    });

    it('sets error message from AppError object on download failure', async () => {
        mockedInvoke.mockRejectedValueOnce({ code: 'Internal', message: 'disk full' });
        const store = createModelDownloadStore();
        await store.download('tiny-en-q5');
        expect(store.error).toBe('disk full');
    });

    it('sets error message from plain string on download failure', async () => {
        mockedInvoke.mockRejectedValueOnce('network timeout');
        const store = createModelDownloadStore();
        await store.download('tiny-en-q5');
        expect(store.error).toBe('network timeout');
    });

    it('marks model as not-downloading after download failure', async () => {
        mockedInvoke.mockRejectedValueOnce({ code: 'Internal', message: 'disk full' });
        const store = createModelDownloadStore();
        await store.download('tiny-en-q5');
        expect(store.downloadingByModel['tiny-en-q5']).toBe(false);
    });

    it('sets error from AppError object on select failure', async () => {
        mockedInvoke.mockRejectedValueOnce({ code: 'NotFound', message: 'model not found' });
        mockedInvoke.mockResolvedValueOnce([]);
        const store = createModelDownloadStore();
        await store.select('tiny-en-q5');
        expect(store.error).toBe('model not found');
    });

    it('surfaces auto-select failure during refresh', async () => {
        mockedInvoke.mockResolvedValueOnce([
            {
                id: 'tiny-en-q5',
                label: 'Tiny',
                file_name: 'ggml-tiny.en-q5_1.bin',
                downloaded: true,
                selected: false,
                size_mb: 31,
                wer: 5.66,
                rtfx: 348,
            },
        ]);
        mockedInvoke.mockRejectedValueOnce({ code: 'Internal', message: 'cannot persist model' });
        const store = createModelDownloadStore();
        await store.refresh();
        expect(store.error).toBe('cannot persist model');
    });

    it('refreshes after successful auto-select', async () => {
        mockedInvoke.mockResolvedValueOnce([
            {
                id: 'tiny-en-q5',
                label: 'Tiny',
                file_name: 'ggml-tiny.en-q5_1.bin',
                downloaded: true,
                selected: false,
                size_mb: 31,
                wer: 5.66,
                rtfx: 348,
            },
        ]);
        mockedInvoke.mockResolvedValueOnce(undefined);
        mockedInvoke.mockResolvedValueOnce([
            {
                id: 'tiny-en-q5',
                label: 'Tiny',
                file_name: 'ggml-tiny.en-q5_1.bin',
                downloaded: true,
                selected: true,
                size_mb: 31,
                wer: 5.66,
                rtfx: 348,
            },
        ]);
        const store = createModelDownloadStore();
        await store.refresh();
        expect(store.models[0].selected).toBe(true);
    });
});
