<script lang="ts">
	let {
		elapsedSeconds = 0,
		class: className = "",
	}: {
		elapsedSeconds?: number;
		class?: string;
	} = $props();

	function format(t: number) {
		const s = Math.max(0, Math.floor(t));
		const m = Math.floor(s / 60);
		const r = s % 60;
		return `${String(m).padStart(2, "0")}:${String(r).padStart(2, "0")}`;
	}

	let text = $derived(format(elapsedSeconds));
</script>

<time
	class="font-data tabular-nums tracking-tight text-on-surface {className}"
	datetime={`PT${Math.floor(elapsedSeconds)}S`}
>
	{text}
</time>
