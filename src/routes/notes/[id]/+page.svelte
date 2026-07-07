<script lang="ts">
	import { page } from '$app/state';
	import NoteEditorView from '@views/note-editor.svelte';
</script>

<NoteEditorView id={page.params.id!} />
