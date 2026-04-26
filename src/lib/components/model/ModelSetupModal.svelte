<script lang="ts">
	import Button from '@lib/components/Button.svelte';
	import Modal from '@lib/components/Modal.svelte';
	import ModelSettingsContent from '@lib/components/model/ModelSettingsContent.svelte';

	let {
		open = false,
		onClose,
	}: {
		open?: boolean;
		onClose?: () => void;
	} = $props();

	let canClose = $state(false);
</script>

<Modal
	{open}
	title="Model setup"
	description="Download and choose a model. Downloaded models stay on this machine."
	showCloseButton={false}
	onClose={onClose}
>
	<ModelSettingsContent showHeader={false} showToast={false} bind:ready={canClose} />

	{#snippet footer()}
		<Button variant="secondary" disabled={!canClose} onclick={onClose}>Close</Button>
	{/snippet}
</Modal>
