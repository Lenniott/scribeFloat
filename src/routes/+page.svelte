<script lang="ts">
	import { goto } from '$app/navigation';
	import { appState } from '@lib/stores/appState.svelte';
	import { openNoteDetail } from '@lib/stores/appActions';
	import HomeView from '@lib/views/home.svelte';
</script>

<HomeView
	items={appState.allItems}
	loading={appState.loading}
	onselect={openNoteDetail}
	onseeall={() => void goto('/notes')}
/>
