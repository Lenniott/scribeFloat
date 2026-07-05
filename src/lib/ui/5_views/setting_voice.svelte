<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import Button from '@components/controls/Button.svelte';
	import ToggleSwitch from '@components/controls/Toggle.svelte';
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
	let enrolling = $state<{ name?: string; locked: boolean } | null>(null);
	let editingSlug = $state('');
	let editingName = $state('');
	let confirmingDelete = $state('');
	let userDisplayName = $state('You');
	let savedUserDisplayName = $state('You');
	let threshold = $state(0.75);
	let voiceLearningEnabled = $state(false);
	let embeddingsRetention = $state<'keep' | 'delete_after_transcript'>('keep');
	let encryptionRequired = $state(true);
	let thresholdSaveTimer: ReturnType<typeof setTimeout> | undefined;

	onMount(async () => {
		await Promise.all([refresh(), loadVoiceSettings()]);
	});

	async function refresh() {
		loadError = '';
		try {
			profiles = await invoke<ProfileSummary[]>('voiceprint_list_profiles');
		} catch (e) {
			loadError = `Could not load voiceprints: ${appErrorMessage(e)}`;
		}
	}

	async function loadVoiceSettings() {
		try {
			const [name, nextThreshold] = await Promise.all([
				invoke<string>('settings_get_user_display_name'),
				invoke<number>('settings_get_voice_similarity_threshold'),
			]);
			const [learningEnabled, retention, encryption] = await Promise.all([
				invoke<boolean>('settings_get_voice_learning_enabled'),
				invoke<'keep' | 'delete_after_transcript'>('settings_get_voice_embeddings_retention'),
				invoke<boolean>('settings_get_voice_embeddings_encryption_required'),
			]);
			userDisplayName = name;
			savedUserDisplayName = name;
			threshold = nextThreshold;
			voiceLearningEnabled = learningEnabled;
			embeddingsRetention = retention;
			encryptionRequired = encryption;
		} catch (e) {
			actionError = `Could not load voice settings: ${appErrorMessage(e)}`;
		}
	}

	function startRename(profile: ProfileSummary) {
		actionError = '';
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
		try {
			await invoke('voiceprint_delete_profile', { slug: profile.slug });
		} catch (e) {
			profiles = previous;
			actionError = `Could not delete profile: ${appErrorMessage(e)}`;
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

	async function saveUserDisplayName() {
		const name = userDisplayName.trim();
		if (!name) {
			userDisplayName = savedUserDisplayName;
			actionError = 'Display name cannot be empty.';
			return;
		}
		actionError = '';
		try {
			await invoke('settings_set_user_display_name', { name });
			userDisplayName = name;
			savedUserDisplayName = name;
		} catch (e) {
			userDisplayName = savedUserDisplayName;
			actionError = `Could not save display name: ${appErrorMessage(e)}`;
		}
	}

	function onThresholdInput(event: Event) {
		threshold = Number((event.currentTarget as HTMLInputElement).value);
		if (thresholdSaveTimer) clearTimeout(thresholdSaveTimer);
		thresholdSaveTimer = setTimeout(() => {
			void saveThreshold();
		}, 300);
	}

	async function saveThreshold() {
		actionError = '';
		try {
			await invoke('settings_set_voice_similarity_threshold', { threshold });
		} catch (e) {
			actionError = `Could not save matching sensitivity: ${appErrorMessage(e)}`;
		}
	}

	async function setVoiceLearningEnabled(enabled: boolean) {
		const previous = voiceLearningEnabled;
		voiceLearningEnabled = enabled;
		actionError = '';
		try {
			await invoke('settings_set_voice_learning_enabled', { enabled });
		} catch (e) {
			voiceLearningEnabled = previous;
			actionError = `Could not save voice learning setting: ${appErrorMessage(e)}`;
		}
	}

	async function setEmbeddingsRetention(retention: 'keep' | 'delete_after_transcript') {
		const previous = embeddingsRetention;
		embeddingsRetention = retention;
		actionError = '';
		try {
			await invoke('settings_set_voice_embeddings_retention', { retention });
		} catch (e) {
			embeddingsRetention = previous;
			actionError = `Could not save voice data setting: ${appErrorMessage(e)}`;
		}
	}

	async function setEncryptionRequired(required: boolean) {
		const previous = encryptionRequired;
		encryptionRequired = required;
		actionError = '';
		try {
			await invoke('settings_set_voice_embeddings_encryption_required', { required });
		} catch (e) {
			encryptionRequired = previous;
			actionError = `Could not save encryption setting: ${appErrorMessage(e)}`;
		}
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
		<h2 class="sf-headline-sm text-fg">Voice</h2>

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
			title="Voiceprints"
			description="Each voiceprint is built from one or more clips. More clips across different distances and mics makes identification more accurate."
		>
			{#snippet action()}
				<Button variant="normal" size="small" onclick={() => (enrolling = { locked: false })}>
					Enroll a voice
				</Button>
			{/snippet}

			{#if profiles.length === 0}
				<div class="rounded-md border border-fill bg-panel px-3 py-4">
					<p class="sf-label-md text-fg">No voiceprints yet.</p>
					<p class="mt-1 sf-label-sm text-fg-dim">
						Add voiceprints to label transcripts by speaker. Start with yourself, then add others as you meet them.
					</p>
					<Button class="mt-3" variant="primary" onclick={() => (enrolling = { locked: false })}>
						Enroll a voice
					</Button>
				</div>
			{:else}
				<SettingsList>
					{#each profiles as profile (profile.slug)}
						<SettingsRow title={profile.name} description={profileMeta(profile)}>
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
										Delete {profile.name}? Segments labelled [{profile.name}] will show as [Other].
									</p>
									<div class="flex justify-end gap-2">
										<Button variant="ghost" size="small" onclick={() => (confirmingDelete = '')}>Cancel</Button>
										<Button variant="destructive" size="small" onclick={() => void deleteProfile(profile)}>Delete</Button>
									</div>
								</div>
							{:else}
								<div class="flex flex-wrap gap-2">
									<Button
										variant="normal"
										size="small"
										onclick={() => (enrolling = { name: profile.name, locked: true })}
									>
										Add print
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
										Delete
									</Button>
								</div>
							{/if}
						</SettingsRow>
					{/each}
				</SettingsList>
			{/if}
		</SettingsSection>

		<SettingsSection title="Speaker labels">
			<SettingsList>
				<SettingsRow
					title="Your display name"
					description={`This name appears in transcripts as [${userDisplayName || 'You'}].`}
				>
					{#snippet control()}
						<div class="w-full sm:w-56">
							<TextField
								label="Your display name"
								bind:value={userDisplayName}
								labelHidden
								onblur={() => void saveUserDisplayName()}
							/>
						</div>
					{/snippet}
				</SettingsRow>

				<SettingsRow
					title="Speaker matching sensitivity"
					description="Lower is more inclusive. Higher only labels very confident matches."
				>
					{#snippet control()}
						<div class="flex w-full min-w-56 flex-col gap-1">
							<div class="flex items-center justify-between sf-meta-sm text-fg-dim">
								<span>Inclusive</span>
								<span>{threshold.toFixed(2)}</span>
								<span>Strict</span>
							</div>
							<input
								type="range"
								min="0"
								max="1"
								step="0.05"
								value={threshold}
								class="w-full accent-brand"
								oninput={onThresholdInput}
								aria-label="Speaker matching sensitivity"
							/>
						</div>
					{/snippet}
				</SettingsRow>
			</SettingsList>
		</SettingsSection>

		<SettingsSection
			title="Voice learning"
			description="These controls prepare transcript-based speaker learning. Automatic profile updates stay off until encrypted voice evidence storage is implemented."
		>
			<SettingsList>
				<SettingsRow
					title="Learn speakers from corrected transcripts"
					description="When this is on, confirmed speaker names may be used later to improve saved voiceprints after quality checks pass."
				>
					{#snippet control()}
						<ToggleSwitch
							checked={voiceLearningEnabled}
							onchange={(next) => void setVoiceLearningEnabled(next)}
							aria-label="Learn speakers from corrected transcripts"
						/>
					{/snippet}
				</SettingsRow>

				<SettingsRow
					title="Keep voice data for future speaker matching"
					description="When off, transcripts can keep text and labels while embedding vectors are removed after processing."
				>
					{#snippet control()}
						<ToggleSwitch
							checked={embeddingsRetention === 'keep'}
							onchange={(next) =>
								void setEmbeddingsRetention(next ? 'keep' : 'delete_after_transcript')}
							aria-label="Keep voice data for future speaker matching"
						/>
					{/snippet}
				</SettingsRow>

				<SettingsRow
					title="Require encryption before learning"
					description="Automatic long-term learning should not run unless stored voice embeddings are encrypted at rest."
				>
					{#snippet control()}
						<ToggleSwitch
							checked={encryptionRequired}
							onchange={(next) => void setEncryptionRequired(next)}
							aria-label="Require encryption before learning"
						/>
					{/snippet}
				</SettingsRow>
			</SettingsList>

			<p class="mt-3 rounded-md border border-fill bg-panel px-3 py-2 sf-label-sm text-fg-dim">
				Voice embeddings stay local. Encryption and transcript-based profile updates are planned next;
				this release only stores your preference.
			</p>
		</SettingsSection>
	</section>
{/if}
