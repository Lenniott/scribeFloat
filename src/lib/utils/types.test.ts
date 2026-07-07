import { describe, it, expect } from 'vitest';
import { appErrorMessage, type AppError } from './types';

describe('appErrorMessage', () => {
    it('returns a plain string unchanged', () => {
        expect(appErrorMessage('something went wrong')).toBe('something went wrong');
    });

    it('extracts message from AppError object', () => {
        const err: AppError = { code: 'NotFound', message: 'record not found' };
        expect(appErrorMessage(err)).toBe('record not found');
    });

    it('extracts message from InvalidInput error', () => {
        const err: AppError = { code: 'InvalidInput', message: 'id cannot be empty' };
        expect(appErrorMessage(err)).toBe('id cannot be empty');
    });

    it('extracts message from Internal error', () => {
        const err: AppError = { code: 'Internal', message: 'unexpected panic' };
        expect(appErrorMessage(err)).toBe('unexpected panic');
    });

    it('falls back to String() for unknown shapes', () => {
        expect(appErrorMessage(42)).toBe('42');
        expect(appErrorMessage(null)).toBe('null');
    });

    it('handles an object without a message field gracefully', () => {
        const result = appErrorMessage({ code: 'Internal' });
        expect(typeof result).toBe('string');
    });
});
