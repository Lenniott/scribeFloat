import '@testing-library/jest-dom';
import { vi } from 'vitest';

// Tauri's APIs are only available inside the desktop app runtime.
// In the test environment (jsdom) we stub every module we import.

vi.mock('@tauri-apps/api/core', () => ({
    invoke: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@tauri-apps/api/event', () => ({
    listen: vi.fn().mockResolvedValue(() => {}),
    emit: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@tauri-apps/api/window', () => ({
    getCurrentWindow: vi.fn(() => ({
        onFocusChanged: vi.fn().mockResolvedValue(() => {}),
    })),
}));
