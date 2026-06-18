<script lang="ts">
	let {
		checked = false,
		label,
		count,
		onchange,
	}: {
		checked?: boolean;
		label: string;
		count: number;
		onchange?: (next: boolean) => void;
	} = $props();
</script>

<label class="flex cursor-pointer items-center gap-2.5 rounded-md px-1 py-1 hover:bg-fill">
	<input
		type="checkbox"
		class="size-4 shrink-0 rounded-sm border border-rim accent-active"
		{checked}
		onchange={(e) => onchange?.(e.currentTarget.checked)}
	/>
	<span class="min-w-0 flex-1 truncate sf-label-md text-fg">{label}</span>
	<span class="sf-meta-sm shrink-0 text-fg-dim">{count}</span>
</label>
