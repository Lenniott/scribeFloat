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
