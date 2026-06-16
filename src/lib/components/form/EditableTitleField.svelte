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
		class="sf-headline-sm h-10 w-full max-w-md rounded-sm border border-rim bg-card px-2 text-fg outline-none"
		onblur={commit}
		{onkeydown}
	/>
{:else}
	<button
		type="button"
		class="sf-headline-sm h-10 w-full max-w-full cursor-text truncate rounded-sm bg-panel px-2 text-left text-fg"
		onclick={beginEdit}
	>
		{value || placeholder}
	</button>
{/if}
