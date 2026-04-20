<script lang="ts">
	import ConfigField from "./ConfigField.svelte";

	let {
		label,
		path = $bindable(""),
		onChange,
	}: {
		label: string;
		path?: string;
		onChange?: () => void;
	} = $props();
</script>

<ConfigField mode="action" {label} bind:value={path} onButtonClick={onChange} buttonLabel="Change" />
