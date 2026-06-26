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
	let enrolling = $state<{ name?: string; locked: boolean } | null>(null);
	let editingSlug = $state('');
	let editingName = $state('');
	let confirmingDelete = $state('');
	let userDisplayName = $state('You');
	let savedUserDisplayName = $state('You');
	let threshold = $state(0.75);
	let thresholdSaveTimer: ReturnType<typeof setTimeout> | undefined;

	type ProfileScore = { profile_name: string; score: number };
	type TestState = 'idle' | 'recording' | 'scored';
	let testState = $state<TestState>('idle');
	let testClipId = $state('');
	let testScores = $state<ProfileScore[]>([]);
	let testError = $state('');

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
			userDisplayName = name;
			savedUserDisplayName = name;
			threshold = nextThreshold;
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

	async function startTest() {
		testError = '';
		testScores = [];
		try {
			testClipId = await invoke<string>('voiceprint_start_clip', { micDeviceId: '' });
			testState = 'recording';
		} catch (e) {
			testError = `Could not start test: ${appErrorMessage(e)}`;
		}
	}

	async function stopTest() {
		try {
			type ClipResult = { accepted: boolean; speech_s: number; purity: number };
			const result = await invoke<ClipResult>('voiceprint_stop_clip', { clipId: testClipId });
			if (!result.accepted) {
				const speechSec = result.speech_s.toFixed(1);
				testError = `Clip too short or noisy to score (${speechSec}s of speech detected). Try speaking clearly for at least 5 seconds.`;
				testState = 'idle';
				return;
			}
			testScores = await invoke<ProfileScore[]>('voiceprint_score_clip', { clipId: testClipId });
			await invoke('voiceprint_discard_clip', { clipId: testClipId });
			testState = 'scored';
		} catch (e) {
			testError = `Test failed: ${appErrorMessage(e)}`;
			testState = 'idle';
		}
	}

	function resetTest() {
		testState = 'idle';
		testScores = [];
		testError = '';
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

		{#if profiles.length > 0}
			<SettingsSection
				title="Test identification"
				description="Record a short clip to see how well your voice matches each enrolled profile."
			>
				{#if testError}
					<p class="rounded-md border border-destructive/40 bg-fill px-3 py-2 sf-label-sm text-destructive">
						{testError}
					</p>
				{/if}

				{#if testState === 'idle'}
					<Button variant="normal" size="small" onclick={() => void startTest()}>
						Start test recording
					</Button>
				{:else if testState === 'recording'}
					<div class="flex items-center gap-3">
						<span class="animate-pulse sf-label-sm text-fg-dim">Recording…</span>
						<Button variant="primary" size="small" onclick={() => void stopTest()}>
							Stop &amp; score
						</Button>
					</div>
				{:else if testState === 'scored'}
					<div class="space-y-3">
						{#each testScores.sort((a, b) => b.score - a.score) as result (result.profile_name)}
							{@const pct = Math.round(result.score * 100)}
							{@const thresholdPct = Math.round(threshold * 100)}
							{@const matches = result.score >= threshold}
							<div class="space-y-1">
								<div class="flex items-baseline justify-between">
									<span class="sf-label-sm text-fg">{result.profile_name}</span>
									<span class="sf-meta-sm {matches ? 'text-brand' : 'text-fg-dim'}">
										{pct}% {matches ? '✓ match' : '· no match'}
									</span>
								</div>
								<div class="relative h-2 w-full overflow-visible rounded-full bg-fill">
									<div
										class="h-2 rounded-full {matches ? 'bg-brand' : 'bg-fg-dim/40'}"
										style="width: {pct}%"
									></div>
									<div
										class="absolute top-1/2 h-3 w-0.5 -translate-y-1/2 bg-fg/50"
										style="left: {thresholdPct}%"
										title="Threshold ({thresholdPct}%)"
									></div>
								</div>
							</div>
						{/each}
						<Button variant="ghost" size="small" onclick={resetTest}>Test again</Button>
					</div>
				{/if}
			</SettingsSection>
		{/if}
	</section>
{/if}
