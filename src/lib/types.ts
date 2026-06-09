/** Typed error returned from Tauri IPC commands. Mirrors Rust `types::AppError`. */
export type AppError =
    | { code: 'NotFound'; message: string }
    | { code: 'InvalidInput'; message: string }
    | { code: 'StateMachine'; message: string }
    | { code: 'Io'; message: string }
    | { code: 'Internal'; message: string };

/** Extract a human-readable string from any Tauri IPC rejection value. */
export function appErrorMessage(e: unknown): string {
    if (typeof e === 'string') return e;
    if (e && typeof e === 'object' && 'message' in e) return String((e as AppError).message);
    return String(e);
}

export type UpdateCheckResult = {
	update_available: boolean;
	latest_version: string;
	current_version: string;
	release_url: string;
	release_notes: string;
};

export type ModelListItem = {
	id: string;
	label: string;
	file_name: string;
	downloaded: boolean;
	selected: boolean;
	size_mb: number;
	wer: number;
	/** Real-time factor (higher = faster); null when not benchmarked. */
	rtfx: number | null;
};

export type ModelProgressPayload = {
	model_id: string;
	progress: number;
	bytes_downloaded: number;
	total_bytes?: number;
};

export type PermissionStatus = {
	kind: string;
	granted: boolean;
	can_request: boolean;
	hint?: string;
};


export function extractErrorMessage(error: unknown, fallback: string): string {
	if (typeof error === 'string' && error.trim()) return error;
	if (error instanceof Error && error.message.trim()) return error.message;
	if (typeof error === 'object' && error !== null) {
		const msg = (error as { message?: unknown }).message;
		if (typeof msg === 'string' && msg.trim()) return msg;
	}
	return fallback;
}
