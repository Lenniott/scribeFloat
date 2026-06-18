<script lang="ts">
	import { appState } from '@stores/appState.svelte';
	import { copyItem, openItem, requestDelete, loadNotes } from '@stores/appActions';
	import NotesView from '@views/notes.svelte';
</script>

<NotesView
	allItems={appState.allItems}
	loading={appState.loading}
	bind:selectedItem={appState.selectedItem}
	oncopy={copyItem}
	onopen={openItem}
	ondelete={requestDelete}
	onrefresh={loadNotes}
	deleting={appState.deleting}
/>
