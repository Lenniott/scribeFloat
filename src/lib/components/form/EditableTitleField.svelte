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
		class="h-7 text-headline-sm w-full max-w-md border-0 border-b border-surface-highest bg-transparent tracking-tight text-on-surface outline-none"
		onblur={commit}
		{onkeydown}
	/>
{:else}
	<button
		type="button"
		class="h-7 text-headline-sm max-w-full cursor-text truncate border-0 bg-transparent tracking-tight text-left text-on-surface hover:text-on-surface/70"
		onclick={beginEdit}
	>
		{value || placeholder}
	</button>
{/if}
