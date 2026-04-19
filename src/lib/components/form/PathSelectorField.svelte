<script lang="ts">
	import Button from "../Button.svelte";
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

<div class="flex flex-col gap-1.5 text-left">
	<span class="text-label-sm font-semibold tracking-wide text-on-surface/80 uppercase">{label}</span>
	<div class="flex min-w-0 items-center gap-2">
		<code
			class="text-label-md min-w-0 flex-1 truncate rounded-md bg-surface-container-lowest px-2 py-2 text-on-surface/90"
			title={path}>{path || "—"}</code
		>
		<Button
			variant='secondary'
			onclick={() => onChange?.()}
		>
			Change
		</Button>
	</div>
</div>
