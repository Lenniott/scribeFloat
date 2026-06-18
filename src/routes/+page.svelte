<script lang="ts">
	import { goto } from '$app/navigation';
	import { appState } from '@stores/appState.svelte';
	import { openNoteDetail } from '@stores/appActions';
	import HomeView from '@views/home.svelte';
</script>

<HomeView
	items={appState.allItems}
	loading={appState.loading}
	onselect={openNoteDetail}
	onseeall={() => void goto('/notes')}
/>
