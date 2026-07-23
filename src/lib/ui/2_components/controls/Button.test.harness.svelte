<script lang="ts">
	import Button from './Button.svelte';

	let {
		onclick,
		disabled = false,
	}: {
		onclick?: (e: MouseEvent) => void;
		disabled?: boolean;
	} = $props();
</script>

<Button {onclick} {disabled}>Click me</Button>
