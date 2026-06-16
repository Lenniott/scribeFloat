/** Shared runtime platform detection for the Tauri webviews. */
export const isWindows =
	typeof navigator !== 'undefined' && /Windows/i.test(navigator.userAgent);

/** Modifier key label for Dictate double-tap instructions (left Alt on Windows, left Control on macOS). */
export const dictateModifierLabel = isWindows ? 'Alt' : 'Control';

const MODIFIER_LABELS: Record<string, { win: string; mac: string }> = {
	cmdorctrl: { win: 'Ctrl', mac: 'Cmd' },
	command: { win: 'Ctrl', mac: 'Cmd' },
	cmd: { win: 'Ctrl', mac: 'Cmd' },
	meta: { win: 'Ctrl', mac: 'Cmd' },
	super: { win: 'Ctrl', mac: 'Cmd' },
	ctrl: { win: 'Ctrl', mac: 'Control' },
	control: { win: 'Ctrl', mac: 'Control' },
	alt: { win: 'Alt', mac: 'Alt' },
	option: { win: 'Alt', mac: 'Alt' },
	shift: { win: 'Shift', mac: 'Shift' },
};

function formatHotkeyPart(part: string): string {
	const token = part.trim();
	if (!token) return '';
	const mapped = MODIFIER_LABELS[token.toLowerCase()];
	if (mapped) return isWindows ? mapped.win : mapped.mac;
	return token;
}

/** Human-readable hotkey for the current OS (e.g. `CmdOrCtrl+Shift+L` → `Cmd + Shift + L` on macOS). */
export function formatHotkeyForDisplay(hotkey: string): string {
	if (!hotkey.trim()) return '';
	return hotkey
		.split('+')
		.map(formatHotkeyPart)
		.filter(Boolean)
		.join(' + ');
}
