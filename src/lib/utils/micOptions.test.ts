import { describe, expect, it } from 'vitest';
import { buildMicOptions, resolveSelectedMic } from './micOptions';

describe('buildMicOptions', () => {
	it('prepends system default before device labels', () => {
		expect(buildMicOptions(['USB Mic', 'Built-in'])).toEqual([
			{ value: '', label: 'System Default' },
			{ value: 'USB Mic', label: 'USB Mic' },
			{ value: 'Built-in', label: 'Built-in' },
		]);
	});
});

describe('resolveSelectedMic', () => {
	it('keeps system default when previous is empty', () => {
		expect(resolveSelectedMic('', ['USB Mic'])).toBe('');
	});

	it('keeps selection when device still listed', () => {
		expect(resolveSelectedMic('USB Mic', ['USB Mic', 'Built-in'])).toBe('USB Mic');
	});

	it('resets to system default when device vanished', () => {
		expect(resolveSelectedMic('USB Mic', ['Built-in'])).toBe('');
	});
});
