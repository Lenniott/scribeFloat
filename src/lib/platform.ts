/** Shared runtime platform detection for the Tauri webviews. */
export const isWindows =
	typeof navigator !== 'undefined' && /Windows/i.test(navigator.userAgent);

/** Modifier key label for Dictate double-tap instructions. */
export const dictateModifierLabel = isWindows ? 'Alt' : 'Ctrl';
