<script lang="ts">
	import LabeledTextField from "@lib/components/form/LabeledTextField.svelte";
	import ToggleSwitch from "@lib/components/form/ToggleSwitch.svelte";

	let webhookUrl = $state("");
	let authHeaderName = $state("x-ob-key");
	let authHeaderValue = $state("");
	let autoSendScribe = $state(false);
	let autoSendDictate = $state(false);
</script>

<section class="space-y-4">
	<h2 class="sf-headline-sm">Webhook</h2>
	<LabeledTextField label="Webhook URL" bind:value={webhookUrl} />
	<LabeledTextField label="Auth header name" bind:value={authHeaderName} />
	<LabeledTextField label="Auth header value" bind:value={authHeaderValue} />
	<div class="flex items-center justify-between">
		<span class="text-body-md">Auto-send Scribe &amp; Transcribe</span>
		<ToggleSwitch bind:checked={autoSendScribe} />
	</div>
	<div class="flex items-center justify-between">
		<span class="text-body-md">Auto-send Dictate</span>
		<ToggleSwitch bind:checked={autoSendDictate} />
	</div>
</section>