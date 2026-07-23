<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import HelpContentRenderer from '@components/content/HelpContentRenderer.svelte';
	import { helpContent } from '@lib/content/helpContent';
	import type { HelpContext } from '@lib/content/helpContent.types';
	import {
		dictateModifierLabel,
		formatHotkeyForDisplay,
		isWindows,
	} from '@utils/platform';

	let context = $state<HelpContext>({
		dictateModifierLabel,
		openRecordHotkey: 'CmdOrCtrl+Shift+L',
		isWindows,
		speakerCaptureRequiresDeviceName: false,
	});

	onMount(async () => {
		const [open] = await invoke<[string, string]>('settings_get_hotkeys').catch(() => [
			'',
			'',
		]);
		const speakerCaptureRequiresDeviceName = await invoke<boolean>(
			'settings_speaker_capture_requires_device_name',
		).catch(() => false);

		context = {
			dictateModifierLabel,
			openRecordHotkey: formatHotkeyForDisplay(open || 'CmdOrCtrl+Shift+L'),
			isWindows,
			speakerCaptureRequiresDeviceName,
		};
	});
</script>

<HelpContentRenderer blocks={helpContent.blocks} {context} />
