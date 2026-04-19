<script lang="ts">
	let {
		value = $bindable("Session"),
		placeholder = "Untitled session",
	}: {
		value?: string;
		placeholder?: string;
	} = $props();

	let editing = $state(false);
	let inputEl = $state<HTMLInputElement | null>(null);

	function beginEdit() {
		editing = true;
		queueMicrotask(() => inputEl?.focus());
	}

	function commit() {
		editing = false;
	}

	function onkeydown(e: KeyboardEvent) {
		if (e.key === "Enter") {
			e.preventDefault();
			commit();
		}
		if (e.key === "Escape") {
			commit();
		}
	}
</script>

{#if editing}
	<input
		bind:this={inputEl}
		bind:value
		type="text"
		{placeholder}
		class="font-data text-headline-sm w-full max-w-md border-0 border-b border-primary bg-transparent tracking-stamped text-on-surface uppercase outline-none"
		onblur={commit}
		{onkeydown}
	/>
{:else}
	<button
		type="button"
		class="font-data text-headline-sm max-w-full cursor-text truncate border-0 bg-transparent tracking-stamped text-left text-on-surface uppercase hover:text-primary"
		onclick={beginEdit}
	>
		{value || placeholder}
	</button>
{/if}
