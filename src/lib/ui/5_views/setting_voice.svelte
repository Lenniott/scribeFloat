<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import Button from '@components/controls/Button.svelte';
	import TextField from '@primitives/form/TextField.svelte';
	import SettingsList from '@sections/SettingList.svelte';
	import SettingsRow from '@components/cards/SettingRow.svelte';
	import SettingsSection from '@primitives/form/SettingsSection.svelte';
	import { appErrorMessage } from '@utils/types';

	type SpeakerNameEntry = {
		name: string;
		slug: string;
	};

	let names = $state<SpeakerNameEntry[]>([]);
	let loadError = $state('');
	let actionError = $state('');
	let newName = $state('');
	let editingSlug = $state('');
	let editingName = $state('');
	let confirmingDelete = $state('');

	onMount(refresh);

	async function refresh() {
		loadError = '';
		try {
			names = await invoke<SpeakerNameEntry[]>('speaker_names_list');
		} catch (e) {
			loadError = `Could not load speaker names: ${appErrorMessage(e)}`;
		}
	}

	async function addName() {
		const name = newName.trim();
		if (!name) return;
		actionError = '';
		try {
			await invoke('speaker_name_save', { name });
			newName = '';
			await refresh();
		} catch (e) {
			actionError = appErrorMessage(e);
		}
	}

	function startRename(entry: SpeakerNameEntry) {
		actionError = '';
		editingSlug = entry.slug;
		editingName = entry.name;
		confirmingDelete = '';
	}

	async function saveRename(slug: string) {
		const name = editingName.trim();
		if (!name) {
			actionError = 'Name cannot be empty.';
			return;
		}
		actionError = '';
		try {
			await invoke('speaker_name_save', { name, previousSlug: slug });
			editingSlug = '';
			await refresh();
		} catch (e) {
			actionError = appErrorMessage(e);
		}
	}

	async function deleteName(slug: string) {
		actionError = '';
		confirmingDelete = '';
		try {
			await invoke('speaker_name_delete', { slug });
			await refresh();
		} catch (e) {
			actionError = appErrorMessage(e);
		}
	}
</script>

<section class="space-y-5">
	{#if loadError}
		<p class="rounded-md border border-destructive/40 bg-fill px-3 py-2 sf-label-sm text-destructive">
			{loadError}
		</p>
	{/if}
	{#if actionError}
		<p class="rounded-md border border-destructive/40 bg-fill px-3 py-2 sf-label-sm text-destructive">
			{actionError}
		</p>
	{/if}

	<SettingsSection
		title="Speaker names"
		description="Names you can assign to speakers in a transcript. Renaming a speaker in any note saves the name here for reuse. Just text — no voice data is stored."
	>
		<div class="flex items-end gap-2">
			<TextField label="Speaker name" bind:value={newName} />
			<Button variant="primary" size="small" disabled={!newName.trim()} onclick={() => void addName()}>
				Add name
			</Button>
		</div>

		{#if names.length === 0}
			<div class="mt-3 rounded-md border border-fill bg-panel px-3 py-4">
				<p class="sf-label-md text-fg">No names yet.</p>
				<p class="mt-1 sf-label-sm text-fg-dim">
					Add people you talk with often, or rename a speaker directly in a transcript.
				</p>
			</div>
		{:else}
			<SettingsList>
				{#each names as entry (entry.slug)}
					<SettingsRow class="bg-card p-2" title={entry.name}>
						{#if editingSlug === entry.slug}
							<div class="flex flex-col gap-2">
								<TextField label="Name" bind:value={editingName} labelHidden />
								<div class="flex justify-end gap-2">
									<Button variant="ghost" size="small" onclick={() => (editingSlug = '')}>Cancel</Button>
									<Button variant="primary" size="small" onclick={() => void saveRename(entry.slug)}>Save</Button>
								</div>
							</div>
						{:else if confirmingDelete === entry.slug}
							<div class="flex flex-col gap-2">
								<p class="sf-label-sm text-fg-dim">
									Remove {entry.name}? Notes already labelled {entry.name} keep their labels.
								</p>
								<div class="flex justify-end gap-2">
									<Button variant="ghost" size="small" onclick={() => (confirmingDelete = '')}>Cancel</Button>
									<Button variant="destructive" size="small" onclick={() => void deleteName(entry.slug)}>
										Remove name
									</Button>
								</div>
							</div>
						{:else}
							<div class="flex flex-wrap gap-2">
								<Button variant="ghost" size="small" onclick={() => startRename(entry)}>Rename</Button>
								<Button
									variant="destructive"
									size="small"
									onclick={() => {
										confirmingDelete = entry.slug;
										editingSlug = '';
									}}
								>
									Remove
								</Button>
							</div>
						{/if}
					</SettingsRow>
				{/each}
			</SettingsList>
		{/if}
	</SettingsSection>
</section>
