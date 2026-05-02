export type ThemeMode = "system" | "dark" | "light";
export type ResolvedTheme = "dark" | "light";

const darkPreference = "(prefers-color-scheme: dark)";

function themeQuery() {
	if (typeof window === "undefined") return undefined;
	return window.matchMedia(darkPreference);
}

export function resolveThemeMode(mode: ThemeMode): ResolvedTheme {
	if (mode === "dark" || mode === "light") return mode;
	return themeQuery()?.matches ? "dark" : "light";
}

/** Clears inline paints from `app.html` boot script so `html { background-color: var(--sf-canvas) }` applies. */
function clearBootInlineThemeStyles() {
	const s = document.documentElement.style;
	s.removeProperty("background-color");
	s.removeProperty("--sf-bg");
	s.removeProperty("--sf-line");
	s.removeProperty("--sf-sk");
}

export function applyThemeMode(mode: ThemeMode) {
	if (typeof document === "undefined") return;
	const resolved = resolveThemeMode(mode);
	document.documentElement.dataset.theme = resolved;
	clearBootInlineThemeStyles();
	try { localStorage.setItem('sf_theme_mode', mode); } catch (_) {}
}

export function watchThemeMode(mode: ThemeMode) {
	applyThemeMode(mode);
	if (mode !== "system") return () => {};

	const query = themeQuery();
	if (!query) return () => {};

	const update = () => applyThemeMode(mode);
	query.addEventListener("change", update);
	return () => query.removeEventListener("change", update);
}
