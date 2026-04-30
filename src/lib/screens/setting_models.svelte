<script lang="ts">
	import ModelSettingsContent from '@components/model/ModelSettingsContent.svelte';

	let {
		ready = $bindable(false),
	}: {
		ready?: boolean;
	} = $props();
</script>

<ModelSettingsContent bind:ready />
