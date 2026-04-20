<script lang="ts">
	import ConfigField, { type ConfigFieldOption } from "./ConfigField.svelte";

	export type DeviceOption = ConfigFieldOption;

	let {
		label,
		options,
		value = $bindable(""),
		id,
	}: {
		label: string;
		options: DeviceOption[];
		value?: string;
		id?: string;
	} = $props();
</script>

<ConfigField mode="select" {label} bind:value {id} {options} />
