<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { openUrl } from '@tauri-apps/plugin-opener';
	import type { UpdateCheckResult } from '$lib/types';

	type UpdateState = 'idle' | 'checking' | 'up_to_date' | 'update_available' | 'error';

	let updateState = $state<UpdateState>('idle');
	let updateResult = $state<UpdateCheckResult | null>(null);
	let updateError = $state('');

	async function checkForUpdates() {
		updateState = 'checking';
		updateResult = null;
		updateError = '';
		try {
			const result = await invoke<UpdateCheckResult>('update_check');
			updateResult = result;
			updateState = result.update_available ? 'update_available' : 'up_to_date';
		} catch (e) {
			updateError = typeof e === 'string' ? e : 'Could not reach update server.';
			updateState = 'error';
		}
	}

	async function openDownloadPage() {
		if (updateResult) await openUrl(updateResult.release_url);
	}
</script>

<section class="space-y-8 max-w-2xl">
	<div>
		<h2 class="sf-headline-sm">Help</h2>
		<p class="mt-1 text-body-sm text-fg-dim">How to use ScribeFloat and what every setting does.</p>
	</div>

	<!-- Updates -->
	<div class="space-y-3">
		<h3 class="font-mono text-label-sm font-normal tracking-stamped text-fg/80 uppercase">Updates</h3>
		{#if updateResult}
			<p class="text-body-sm text-fg-dim">
				Current version: <code class="font-mono text-label-sm bg-fill px-1 rounded">{updateResult.current_version}</code>
			</p>
		{/if}
		<div class="flex items-center gap-3">
			<button
				class="sf-button-secondary text-body-sm"
				onclick={checkForUpdates}
				disabled={updateState === 'checking'}
			>
				{updateState === 'checking' ? 'Checking…' : 'Check for updates'}
			</button>
			{#if updateState === 'up_to_date'}
				<span class="text-body-sm text-fg-dim">You're on the latest version.</span>
			{/if}
		</div>
		{#if updateState === 'update_available' && updateResult}
			<div class="rounded-md border border-card bg-fill p-3 space-y-2">
				<p class="text-body-sm text-fg font-medium">
					Version {updateResult.latest_version} is available
				</p>
				{#if updateResult.release_notes}
					<p class="text-body-sm text-fg-dim">{updateResult.release_notes}</p>
				{/if}
				<button
					class="sf-button-primary text-body-sm"
					onclick={openDownloadPage}
				>
					Open download page
				</button>
			</div>
		{/if}
		{#if updateState === 'error'}
			<p class="text-body-sm text-fg-dim">Could not check for updates: {updateError}</p>
		{/if}
	</div>

	<!-- Scribe -->
	<div class="space-y-2">
		<h3 class="font-mono text-label-sm font-normal tracking-stamped text-fg/80 uppercase">Scribe</h3>
		<p class="text-body-sm text-fg">
			Scribe records your microphone and transcribes the audio into a timestamped Markdown file saved in your save folder.
		</p>
		<ul class="space-y-1 text-body-sm text-fg list-disc pl-5">
			<li>Open Scribe from the menu bar icon or press the <strong>Open Scribe hotkey</strong> (shown in General settings — <code class="font-mono text-label-sm bg-fill px-1 rounded">CmdOrCtrl+Shift+S</code> by default).</li>
			<li>Press <strong>Record</strong> to start. Add timestamped notes while recording if you like.</li>
			<li>Press <strong>Stop & Save</strong> — ScribeFloat transcribes the audio and saves a <code class="font-mono text-label-sm bg-fill px-1 rounded">.md</code> file. The recording is deleted once the transcript is confirmed saved.</li>
			<li>Enable <strong>Speaker capture</strong> to also record system audio (e.g. for calls and meetings). Mic lines are prefixed <code class="font-mono text-label-sm bg-fill px-1 rounded">in:</code> and speaker lines <code class="font-mono text-label-sm bg-fill px-1 rounded">out:</code> in the transcript.</li>
			<li>If no model is installed, the WAV file is kept and a button appears to open it in Transcribe later.</li>
		</ul>
	</div>

	<!-- Dictate -->
	<div class="space-y-2">
		<h3 class="font-mono text-label-sm font-normal tracking-stamped text-fg/80 uppercase">Dictate</h3>
		<p class="text-body-sm text-fg">
			Dictate is a floating hotkey-driven voice input. Audio is processed entirely in memory — nothing is written to disk.
		</p>
		<ul>
			<li><strong>Double-tap Ctrl</strong> to toggle recording on/off (useful for longer dictations).</li>
			<li>If Accessibility permission is granted, the text is pasted automatically via <code class="font-mono text-label-sm bg-fill px-1 rounded">Cmd/Ctrl+V</code>. Otherwise it goes to the clipboard.</li>
			<li>Enable <strong>Press Enter after dictate</strong> in General settings to send an Enter keystroke after the paste — handy for messaging apps.</li>
			<li>Each successful dictation is appended to <code class="font-mono text-label-sm bg-fill px-1 rounded">dictate.jsonl</code> in your save folder.</li>
		</ul>
	</div>

	<!-- Transcribe -->
	<div class="space-y-2">
		<h3 class="font-mono text-label-sm font-normal tracking-stamped text-fg/80 uppercase">Transcribe</h3>
		<p class="text-body-sm text-fg">
			Transcribe converts existing audio files to text. Open it from the menu bar icon.
		</p>
		<ul class="space-y-1 text-body-sm text-fg list-disc pl-5">
			<li>Drag a <strong>WAV, MP3, M4A, or FLAC</strong> file onto the panel (or use the file picker).</li>
			<li>Choose an output folder and press <strong>Transcribe</strong>.</li>
			<li>If the file is a dual-source Scribe session folder (contains <code class="font-mono text-label-sm bg-fill px-1 rounded">mic.wav</code> + <code class="font-mono text-label-sm bg-fill px-1 rounded">session.json</code>), the dual-source merge runs automatically.</li>
		</ul>
	</div>

	<!-- Models -->
	<div class="space-y-2">
		<h3 class="font-mono text-label-sm font-normal tracking-stamped text-fg/80 uppercase">Models</h3>
		<p class="text-body-sm text-fg">
			ScribeFloat uses <strong>OpenAI Whisper</strong> running entirely on your device. Download a model once; it works offline forever. You can set a separate model for Scribe and Dictate in Settings → Models.
		</p>
		<div class="overflow-hidden rounded-md border border-card text-body-sm">
			<table class="w-full">
				<thead class="bg-fill">
					<tr>
						<th class="px-3 py-2 text-left font-mono text-label-sm font-normal tracking-stamped text-fg/70 uppercase">Model</th>
						<th class="px-3 py-2 text-left font-mono text-label-sm font-normal tracking-stamped text-fg/70 uppercase">Size</th>
						<th class="px-3 py-2 text-left font-mono text-label-sm font-normal tracking-stamped text-fg/70 uppercase">Speed</th>
						<th class="px-3 py-2 text-left font-mono text-label-sm font-normal tracking-stamped text-fg/70 uppercase">Accuracy</th>
					</tr>
				</thead>
				<tbody class="divide-y divide-card">
					<tr>
						<td class="px-3 py-2 text-fg">Tiny</td>
						<td class="px-3 py-2 text-fg-dim">~75 MB</td>
						<td class="px-3 py-2 text-fg-dim">Fastest</td>
						<td class="px-3 py-2 text-fg-dim">Basic</td>
					</tr>
					<tr>
						<td class="px-3 py-2 text-fg">Base</td>
						<td class="px-3 py-2 text-fg-dim">~145 MB</td>
						<td class="px-3 py-2 text-fg-dim">Fast</td>
						<td class="px-3 py-2 text-fg-dim">Good</td>
					</tr>
					<tr>
						<td class="px-3 py-2 text-fg">Small</td>
						<td class="px-3 py-2 text-fg-dim">~460 MB</td>
						<td class="px-3 py-2 text-fg-dim">Moderate</td>
						<td class="px-3 py-2 text-fg-dim">Better — recommended</td>
					</tr>
					<tr>
						<td class="px-3 py-2 text-fg">Medium</td>
						<td class="px-3 py-2 text-fg-dim">~1.5 GB</td>
						<td class="px-3 py-2 text-fg-dim">Slow</td>
						<td class="px-3 py-2 text-fg-dim">Best</td>
					</tr>
				</tbody>
			</table>
		</div>
	</div>

	<!-- Settings reference -->
	<div class="space-y-2">
		<h3 class="font-mono text-label-sm font-normal tracking-stamped text-fg/80 uppercase">Settings reference</h3>
		<div class="overflow-hidden rounded-md border border-card text-body-sm">
			<table class="w-full">
				<thead class="bg-fill">
					<tr>
						<th class="px-3 py-2 text-left font-mono text-label-sm font-normal tracking-stamped text-fg/70 uppercase">Setting</th>
						<th class="px-3 py-2 text-left font-mono text-label-sm font-normal tracking-stamped text-fg/70 uppercase">What it does</th>
					</tr>
				</thead>
				<tbody class="divide-y divide-card">
					<tr>
						<td class="px-3 py-2 text-fg font-medium">Theme</td>
						<td class="px-3 py-2 text-fg-dim">Light, dark, or follow the OS setting.</td>
					</tr>
					<tr>
						<td class="px-3 py-2 text-fg font-medium">Default save folder</td>
						<td class="px-3 py-2 text-fg-dim">Where transcripts and Dictate history are saved.</td>
					</tr>
					<tr>
						<td class="px-3 py-2 text-fg font-medium">Open transcripts with</td>
						<td class="px-3 py-2 text-fg-dim">App used to open <code class="font-mono text-label-sm bg-fill px-1 rounded">.md</code> files after transcription. Leave blank to use the system default.</td>
					</tr>
					<tr>
						<td class="px-3 py-2 text-fg font-medium">Open Scribe hotkey</td>
						<td class="px-3 py-2 text-fg-dim">Global shortcut to show or bring back the Scribe panel from anywhere. Shown in General settings — fixed and not currently editable.</td>
					</tr>
					<tr>
						<td class="px-3 py-2 text-fg font-medium">Capture speaker by default</td>
						<td class="px-3 py-2 text-fg-dim">Pre-enables dual-source (mic + speaker) whenever Scribe opens.</td>
					</tr>
					<tr>
						<td class="px-3 py-2 text-fg font-medium">Press Enter after dictate</td>
						<td class="px-3 py-2 text-fg-dim">Sends an Enter keystroke immediately after the dictated text is pasted. Useful in messaging and search apps.</td>
					</tr>
					<tr>
						<td class="px-3 py-2 text-fg font-medium">Speaker capture device name</td>
						<td class="px-3 py-2 text-fg-dim">Exact device name for system audio capture. On macOS this is usually <code class="font-mono text-label-sm bg-fill px-1 rounded">BlackHole 2ch</code>. Leave blank to use the system default.</td>
					</tr>
					<tr>
						<td class="px-3 py-2 text-fg font-medium">Input / Output label</td>
						<td class="px-3 py-2 text-fg-dim">Prefix labels for each audio source in dual-source transcripts. Defaults: <code class="font-mono text-label-sm bg-fill px-1 rounded">in:</code> for mic, <code class="font-mono text-label-sm bg-fill px-1 rounded">out:</code> for speaker.</td>
					</tr>
				</tbody>
			</table>
		</div>
	</div>
</section>
