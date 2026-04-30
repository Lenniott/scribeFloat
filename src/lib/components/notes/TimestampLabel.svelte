<script lang="ts">
	let {
		at,
		class: className = "",
	}: {
		at: number;
		class?: string;
	} = $props();

	function format(ms: number) {
		const totalSeconds = Math.floor(ms / 1000);
		const hours = Math.floor(totalSeconds / 3600);
		const minutes = Math.floor((totalSeconds % 3600) / 60);
		const seconds = totalSeconds % 60;
		return `${String(hours).padStart(2, '0')}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
	}

	let text = $derived(format(at));
</script>

<span class="font-mono text-label-sm font-normal tabular-nums tracking-stamped text-on-surface/55 {className}">{text}</span>
