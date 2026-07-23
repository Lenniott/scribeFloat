export type MicOption = { value: string; label: string };

export function buildMicOptions(devices: string[]): MicOption[] {
	return [
		{ value: '', label: 'System Default' },
		...devices.map((d) => ({ value: d, label: d })),
	];
}

/** Keep previous selection when still present; reset to system default if the device vanished. */
export function resolveSelectedMic(previous: string, devices: string[]): string {
	if (previous === '' || devices.includes(previous)) return previous;
	return '';
}
