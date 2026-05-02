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
		class="h-10 px-2 bg-panel rounded-sm text-headline-sm w-full max-w-md tracking-tight text-fg outline-none border border-rim"
		onblur={commit}
		{onkeydown}
	/>
{:else}
	<button
		type="button"
		class="h-10 px-2 bg-panel rounded-sm text-headline-sm w-full max-w-full cursor-text truncate tracking-tight text-left text-fg"
		onclick={beginEdit}
	>
		{value || placeholder}
	</button>
{/if}
