import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { createModelDownloadStore } from './modelDownload.svelte';

const mockedInvoke = vi.mocked(invoke);

describe('createModelDownloadStore', () => {
    beforeEach(() => {
        vi.clearAllMocks();
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
        // first invoke (refresh → model_list) succeeds with empty list
        mockedInvoke.mockResolvedValueOnce([]);
        // second invoke (model_select) fails
        mockedInvoke.mockRejectedValueOnce({ code: 'NotFound', message: 'model not found' });
        // third invoke (refresh after select) succeeds
        mockedInvoke.mockResolvedValueOnce([]);
        const store = createModelDownloadStore();
        await store.select('tiny-en-q5');
        expect(store.error).toBe('model not found');
    });
});
