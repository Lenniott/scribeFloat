import { invoke } from '@tauri-apps/api/core';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';

export async function copyTranscript(path: string): Promise<void> {
	const content = await invoke<string>('scribe_read_transcript', { path });
	await writeText(extractTranscriptBody(content));
}

function extractTranscriptBody(content: string): string {
	const transcriptHeading = content.match(/^## Transcript\s*$/m);
	if (transcriptHeading?.index == null) return content.trim();

	const bodyStart = transcriptHeading.index + transcriptHeading[0].length;
	const afterHeading = content.slice(bodyStart).replace(/^\s+/, '');
	const nextSection = afterHeading.search(/^##\s+/m);
	const transcript = nextSection === -1 ? afterHeading : afterHeading.slice(0, nextSection);

	return transcript.trim();
}
