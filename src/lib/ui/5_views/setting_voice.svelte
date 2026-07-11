<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import Button from '@components/controls/Button.svelte';
	import TextField from '@primitives/form/TextField.svelte';
	import SettingsList from '@sections/SettingList.svelte';
	import SettingsRow from '@components/cards/SettingRow.svelte';
	import SettingsSection from '@primitives/form/SettingsSection.svelte';
	import VoiceEnrollmentStep from '@sections/onboarding/VoiceEnrollmentStep.svelte';
	import { appErrorMessage } from '@utils/types';

	type ProfileSummary = {
		slug: string;
		name: string;
		mic_device_id: string | null;
		mic_device_label: string | null;
		sample_count: number;
		updated_at: string;
	};

	let profiles = $state<ProfileSummary[]>([]);
	let loadError = $state('');
	let actionError = $state('');
	let actionMessage = $state('');
	let enrolling = $state<{ name?: string; locked: boolean } | null>(null);
	let editingSlug = $state('');
	let editingName = $state('');
	let confirmingDelete = $state('');
	let confirmingDeleteAllProfiles = $state(false);

	onMount(refresh);

	async function refresh() {
		loadError = '';
		try {
			profiles = await invoke<ProfileSummary[]>('voiceprint_list_profiles');
		} catch (e) {
			loadError = `Could not load voiceprints: ${appErrorMessage(e)}`;
		}
	}

	function startRename(profile: ProfileSummary) {
		actionError = '';
		actionMessage = '';
		editingSlug = profile.slug;
		editingName = profile.name;
		confirmingDelete = '';
	}

	async function saveRename(slug: string) {
		const name = editingName.trim();
		if (!name) {
			actionError = 'Profile name cannot be empty.';
			return;
		}
		const previous = profiles;
		profiles = profiles.map((profile) =>
			profile.slug === slug ? { ...profile, name } : profile,
		);
		editingSlug = '';
		actionError = '';
		actionMessage = '';
		try {
			await invoke('voiceprint_rename_profile', { slug, name });
			await refresh();
		} catch (e) {
			profiles = previous;
			actionError = `Could not rename profile: ${appErrorMessage(e)}`;
		}
	}

	async function deleteProfile(profile: ProfileSummary) {
		const previous = profiles;
		profiles = profiles.filter((item) => item.slug !== profile.slug);
		confirmingDelete = '';
		actionError = '';
		actionMessage = '';
		try {
			await invoke('voiceprint_delete_profile', { slug: profile.slug });
		} catch (e) {
			profiles = previous;
			actionError = `Could not delete profile: ${appErrorMessage(e)}`;
		}
	}

	async function deleteAllProfiles() {
		const previous = profiles;
		profiles = [];
		confirmingDeleteAllProfiles = false;
		actionError = '';
		actionMessage = '';
		try {
			const deleted = await invoke<number>('voiceprint_delete_all_profiles');
			actionMessage = `Removed ${deleted} saved ${deleted === 1 ? 'voice' : 'voices'}.`;
			await refresh();
		} catch (e) {
			profiles = previous;
			actionError = `Could not remove voices: ${appErrorMessage(e)}`;
		}
	}

	function formatDate(value: string): string {
		const date = new Date(value);
		if (Number.isNaN(date.getTime())) return 'unknown';
		return date.toLocaleDateString(undefined, {
			year: 'numeric',
			month: 'short',
			day: '2-digit',
		});
	}

	function profileMeta(profile: ProfileSummary): string {
		const mic = profile.mic_device_label ?? profile.mic_device_id ?? 'Unknown mic';
		const clips = `${profile.sample_count} ${profile.sample_count === 1 ? 'clip' : 'clips'}`;
		return `${mic} · ${clips} · Updated ${formatDate(profile.updated_at)}`;
	}

	async function finishEnrollment() {
		enrolling = null;
		await refresh();
	}
</script>

{#if enrolling}
	<div class="flex min-h-[480px] flex-col rounded-md border border-fill bg-panel p-4">
		<VoiceEnrollmentStep
			isFirstTime={false}
			prefilledName={enrolling.name ?? 'You'}
			lockedName={enrolling.locked}
			onBack={() => (enrolling = null)}
			onNext={() => void finishEnrollment()}
		/>
	</div>
{:else}
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
		{#if actionMessage}
			<p class="rounded-md border border-fill bg-panel px-3 py-2 sf-label-sm text-fg-dim">
				{actionMessage}
			</p>
		{/if}

		<SettingsSection
			title="Voiceprints"
			description="Each voice is built from one or more clips. More clips across different distances and mics makes identification more accurate."
		>
			{#snippet action()}
				<Button variant="normal" size="small" onclick={() => (enrolling = { locked: false })}>
					Add voice
				</Button>
			{/snippet}

			{#if profiles.length === 0}
				<div class="rounded-md border border-fill bg-panel px-3 py-4">
					<p class="sf-label-md text-fg">No voices yet.</p>
					<p class="mt-1 sf-label-sm text-fg-dim">
						Add voices to label transcripts by speaker. Start with yourself, then add others as you meet them.
					</p>
					<Button class="mt-3" variant="primary" onclick={() => (enrolling = { locked: false })}>
						Add voice
					</Button>
				</div>
			{:else}
				<SettingsList>
					{#each profiles as profile (profile.slug)}
						<SettingsRow class="bg-card p-2" title={profile.name} description={profileMeta(profile)}>
							{#if editingSlug === profile.slug}
								<div class="flex flex-col gap-2">
									<TextField label="Profile name" bind:value={editingName} labelHidden />
									<div class="flex justify-end gap-2">
										<Button variant="ghost" size="small" onclick={() => (editingSlug = '')}>Cancel</Button>
										<Button variant="primary" size="small" onclick={() => void saveRename(profile.slug)}>Save</Button>
									</div>
								</div>
							{:else if confirmingDelete === profile.slug}
								<div class="flex flex-col gap-2">
									<p class="sf-label-sm text-fg-dim">
										Remove {profile.name}? Segments labelled [{profile.name}] will show as [Other].
									</p>
									<div class="flex justify-end gap-2">
										<Button variant="ghost" size="small" onclick={() => (confirmingDelete = '')}>Cancel</Button>
										<Button variant="destructive" size="small" onclick={() => void deleteProfile(profile)}>Remove</Button>
									</div>
								</div>
							{:else}
								<div class="flex flex-wrap gap-2">
									<Button
										variant="normal"
										size="small"
										onclick={() => (enrolling = { name: profile.name, locked: true })}
									>
										Refine voice
									</Button>
									<Button variant="ghost" size="small" onclick={() => startRename(profile)}>
										Rename
									</Button>
									<Button
										variant="destructive"
										size="small"
										onclick={() => {
											confirmingDelete = profile.slug;
											editingSlug = '';
										}}
									>
										Remove voice
									</Button>
								</div>
							{/if}
						</SettingsRow>
					{/each}
				</SettingsList>
				<div class="mt-3 flex flex-col gap-2 rounded-md border border-fill bg-panel px-3 py-3 sm:flex-row sm:items-center sm:justify-between">
					<div>
						<p class="sf-label-md text-fg">Bulk remove voices</p>
						<p class="sf-label-sm text-fg-dim">
							This removes saved profile vectors. Transcript text and labels stay in place.
						</p>
					</div>
					{#if confirmingDeleteAllProfiles}
						<div class="flex shrink-0 gap-2">
							<Button variant="ghost" size="small" onclick={() => (confirmingDeleteAllProfiles = false)}>Cancel</Button>
							<Button variant="destructive" size="small" onclick={() => void deleteAllProfiles()}>Remove all</Button>
						</div>
					{:else}
						<Button
							variant="destructive"
							size="small"
							onclick={() => (confirmingDeleteAllProfiles = true)}
						>
							Remove all
						</Button>
					{/if}
				</div>
			{/if}
		</SettingsSection>
	</section>
{/if}
