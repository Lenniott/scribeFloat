<script lang="ts">
	import { onMount } from 'svelte';
	import ScribeScreen from '@lib/screens/scribe.svelte';
	import ScribeProcessingScreen from '@lib/screens/scribe-processing.svelte';

	type AppScreen = 'recording' | 'processing';

	let {
		onclose,
		registerLeaveGuard,
		visitKey = 0,
	}: {
		onclose?: () => void;
		registerLeaveGuard?: (handler: (proceed: () => void) => void) => void;
		visitKey?: number;
	} = $props();

	let appScreen = $state<AppScreen>('recording');
	let processingTitle = $state('Recording');

	let recordingLeaveHandler: ((proceed: () => void) => void) | null = null;
	let processingLeaveHandler: ((proceed: () => void) => void) | null = null;

	function beginProcessing(title: string) {
		processingTitle = title || 'Recording';
		appScreen = 'processing';
		recordingLeaveHandler = null;
	}

	function returnToRecording() {
		appScreen = 'recording';
		processingLeaveHandler = null;
	}

	function requestLeave(proceed: () => void) {
		if (appScreen === 'processing' && processingLeaveHandler) {
			processingLeaveHandler(proceed);
			return;
		}
		if (recordingLeaveHandler) {
			recordingLeaveHandler(proceed);
			return;
		}
		proceed();
	}

	onMount(() => {
		registerLeaveGuard?.(requestLeave);
	});
</script>

<div class="flex h-full min-h-0 flex-col overflow-hidden">
	{#if appScreen === 'processing'}
		<ScribeProcessingScreen
			embedded
			title={processingTitle}
			onRecordAgain={returnToRecording}
			registerLeaveHandler={(handler) => {
				processingLeaveHandler = handler;
			}}
		/>
	{:else}
		<ScribeScreen
			embedded
			{visitKey}
			processingStart={beginProcessing}
			registerLeaveHandler={(handler) => {
				recordingLeaveHandler = handler;
			}}
		/>
	{/if}
</div>
