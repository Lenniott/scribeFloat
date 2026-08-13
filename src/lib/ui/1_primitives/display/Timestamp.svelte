<script lang="ts">
	let {
		at,
		class: className = "",
	}: {
		at: number;
		class?: string;
	} = $props();

	function format(ms: number) {
		return new Date(ms).toLocaleTimeString(undefined, {
			hour: '2-digit',
			minute: '2-digit',
			second: '2-digit',
		});
	}

	let text = $derived(format(at));
</script>

<span class="sf-meta-sm text-fg-dim {className}">{text}</span>
