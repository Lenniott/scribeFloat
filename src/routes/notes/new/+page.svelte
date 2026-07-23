<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { invoke } from '@tauri-apps/api/core';

	onMount(async () => {
		const id = await invoke<string>('note_create_empty');
		await goto(`/notes/${id}`, { replaceState: true });
	});
</script>
