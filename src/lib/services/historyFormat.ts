export function formatShortDate(iso: string): string {
	const date = new Date(iso);
	return date.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
}

export function formatDurationFromSecs(secs: number): string {
	if (secs <= 0) return '0s';
	const h = Math.floor(secs / 3600);
	const m = Math.floor((secs % 3600) / 60);
	const s = secs % 60;
	if (h > 0) return m > 0 ? `${h}h ${m}m` : `${h}h`;
	if (m > 0) return s > 0 ? `${m}m ${s}s` : `${m}m`;
	return `${s}s`;
}

export function formatDurationFromMs(ms: number): string {
	return formatDurationFromSecs(Math.floor(ms / 1000));
}

export function formatWeekDuration(secs: number | null | undefined): string {
	if (secs == null || secs <= 0) return '—';
	return formatDurationFromSecs(secs);
}
