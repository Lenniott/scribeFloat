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
		class="sf-input sf-headline-sm max-w-md"
		onblur={commit}
		{onkeydown}
	/>
{:else}
	<button
		type="button"
		class="sf-headline-sm w-full max-w-full cursor-text truncate border-0 bg-transparent p-0 text-left text-fg hover:opacity-80"
		onclick={beginEdit}
	>
		{value || placeholder}
	</button>
{/if}
