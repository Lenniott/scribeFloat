<script lang="ts">
	let {
		at,
		class: className = "",
	}: {
		at: number | Date;
		class?: string;
	} = $props();

	function format(ts: number | Date) {
		const d = ts instanceof Date ? ts : new Date(ts);
		return d.toLocaleTimeString(undefined, {
			hour: "2-digit",
			minute: "2-digit",
			second: "2-digit",
			hour12: false,
		});
	}

	let text = $derived(format(at));
</script>

<span class="text-label-sm font-semibold tabular-nums text-on-surface/55 {className}">{text}</span>
