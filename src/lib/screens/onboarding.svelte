<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import Button from '@lib/components/Button.svelte';
	import HotkeyCaptureField from '@lib/components/form/HotkeyCaptureField.svelte';
	import LabeledTextField from '@lib/components/form/LabeledTextField.svelte';
	import PathSelectorField from '@lib/components/form/PathSelectorField.svelte';
	import ModelSetupModal from '@lib/components/model/ModelSetupModal.svelte';
	import { createModelDownloadStore } from '$lib/stores/modelDownload.svelte';
	import { type PermissionStatus, extractErrorMessage } from '$lib/types';

	let { onComplete }: { onComplete?: () => void } = $props();

	const modelStore = createModelDownloadStore();
	let modelUnlisteners: (() => void)[] = [];

	let permissions = $state<PermissionStatus[]>([]);
	let justGrantedByKind = $state<Record<string, boolean>>({});
	let outputPath = $state('');
	let openHotkey = $state('');
	let dictateHotkey = $state('');
	let inputLabel = $state('Mic');
	let outputLabel = $state('Speaker');
	let saveError = $state('');
	let gateError = $state('');
	let isRefreshing = $state(false);
	let requestingKind = $state<string | null>(null);
	let modelSetupOpen = $state(false);
	let step = $state<1 | 2 | 3 | 4>(1);

	const modelReady = $derived(modelStore.models.some((m) => m.downloaded && m.selected));
	const canCloseModelSetup = $derived(modelStore.models.some((m) => m.selected && m.downloaded));
	const microphoneReady = $derived(
		permissions.find((p) => p.kind === 'microphone')?.granted ?? false,
	);
	const outputPathReady = $derived(Boolean(outputPath.trim()));
	const hotkeysReady = $derived(Boolean(openHotkey.trim() && dictateHotkey.trim()));
	const allReady = $derived(modelReady && microphoneReady && outputPathReady && hotkeysReady);

	const stepStatuses = $derived([
		{
			id: 1 as const,
			title: 'Model',
			description: modelReady ? 'Ready' : 'Download and select one model',
			complete: modelReady,
		},
		{
			id: 2 as const,
			title: 'Permissions',
			description: microphoneReady ? 'Microphone ready' : 'Grant microphone access',
			complete: microphoneReady,
		},
		{
			id: 3 as const,
			title: 'Output',
			description: outputPathReady ? 'Ready' : 'Choose output folder',
			complete: outputPathReady,
		},
		{
			id: 4 as const,
			title: 'Hotkeys',
			description: hotkeysReady ? 'Ready' : 'Set scribe and dictate hotkeys',
			complete: hotkeysReady,
		},
	]);

	async function refreshPermissions() {
		const previous = permissions;
		permissions = await invoke<PermissionStatus[]>('settings_permissions_status').catch((e) => {
			gateError = extractErrorMessage(e, 'Could not load permission status.');
			return [];
		});
		const previousMap = new Map(previous.map((p) => [p.kind, p.granted]));
		for (const permission of permissions) {
			if (!previousMap.get(permission.kind) && permission.granted) {
				justGrantedByKind = { ...justGrantedByKind, [permission.kind]: true };
				setTimeout(() => {
					justGrantedByKind = { ...justGrantedByKind, [permission.kind]: false };
				}, 3000);
			}
		}
	}

	async function refreshConfig() {
		outputPath = await invoke<string>('settings_get_output_path').catch((e) => {
			gateError = extractErrorMessage(e, 'Could not load output path.');
			return '';
		});
		const [open, dictate] = await invoke<[string, string]>('settings_get_hotkeys').catch((e) => {
			gateError = extractErrorMessage(e, 'Could not load hotkeys.');
			return ['', ''];
		});
		openHotkey = open;
		dictateHotkey = dictate;
		const [inLabel, outLabel] = await invoke<[string, string]>('settings_get_input_labels').catch(
			(e) => {
				gateError = extractErrorMessage(e, 'Could not load labels.');
				return ['Mic', 'Speaker'];
			},
		);
		inputLabel = inLabel;
		outputLabel = outLabel;
	}

	async function refreshAllStatus() {
		isRefreshing = true;
		gateError = '';
		await Promise.all([modelStore.refresh(), refreshPermissions(), refreshConfig()]);
		isRefreshing = false;
	}

	async function completeOnboarding() {
		saveError = '';
		await invoke('settings_complete_onboarding')
			.then(() => onComplete?.())
			.catch((e) => {
				saveError = extractErrorMessage(e, 'Could not finish onboarding.');
			});
	}

	async function grantPermission(kind: string) {
		requestingKind = kind;
		saveError = '';
		if (kind === 'microphone') {
			// getUserMedia triggers the native macOS permission dialog reliably.
			// A CPAL-based probe can race with the OS dialog.
			try {
				const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
				stream.getTracks().forEach((t) => t.stop());
			} catch {
				// Denied or unavailable — status refresh reflects reality.
			}
		} else {
			await invoke('settings_permissions_open', { kind }).catch((e) => {
				saveError = extractErrorMessage(e, `Could not open settings for ${kind.replace('_', ' ')}.`);
			});
		}
		await refreshPermissions();
		requestingKind = null;
	}

	async function saveOutputPath() {
		saveError = '';
		await invoke('settings_set_output_path', { path: outputPath }).catch((e) => {
			saveError = extractErrorMessage(e, 'Could not save output path.');
		});
		await refreshConfig();
	}

	async function saveHotkeysAndLabels() {
		saveError = '';
		await invoke('settings_set_hotkeys', { openScribe: openHotkey, dictate: dictateHotkey }).catch(
			(e) => {
				saveError = extractErrorMessage(e, 'Could not save hotkeys.');
			},
		);
		await invoke('settings_set_input_labels', { inputLabel, outputLabel }).catch((e) => {
			saveError = extractErrorMessage(e, 'Could not save labels.');
		});
		await refreshConfig();
	}

	async function selectModel(modelId: string) {
		await modelStore.select(modelId);
	}

	async function closeModelSetup() {
		modelSetupOpen = false;
	}

	let unlistenFocus: (() => void) | undefined;

	onMount(async () => {
		modelUnlisteners = await modelStore.subscribe();
		await refreshAllStatus();
		unlistenFocus = await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
			if (focused && step === 2) refreshPermissions();
		});
	});

	onDestroy(() => {
		unlistenFocus?.();
		modelUnlisteners.forEach((u) => u());
	});

</script>

<div class="mx-auto flex h-screen w-full max-w-3xl flex-col gap-6 p-6 text-on-surface">
	<header class="space-y-1">
		<h1 class="text-title-lg font-semibold">Welcome to Liscribe</h1>
		<p class="text-body-sm text-on-surface/70">
			Finish setup once: choose model, check permissions, confirm output path, then configure hotkeys.
		</p>
	</header>

	<nav class="grid gap-2 sm:grid-cols-2">
		{#each stepStatuses as stepStatus (stepStatus.id)}
			<button
				type="button"
				class={`rounded-md border px-3 py-2 text-left transition ${
					stepStatus.id === step
						? 'border-secondary bg-secondary/10'
						: stepStatus.complete
							? 'border-green/40 bg-green/2'
							: 'border-surface-container bg-surface'
				}`}
				onclick={() => (step = stepStatus.id)}
			>
				<div class="flex items-center justify-between gap-3">
					<p class="text-label-sm font-semibold">{stepStatus.id}. {stepStatus.title}</p>
					<span class={`text-label-sm ${stepStatus.complete ? stepStatus.id === step? 'text-secondary':'text-green' : 'text-on-surface/70'}`}>
						{stepStatus.complete ? 'Done' : 'Pending'}
					</span>
				</div>
				<p class="text-label-sm text-on-surface/70">{stepStatus.description}</p>
			</button>
		{/each}
	</nav>

	<section class="rounded-md border border-surface-container p-4">
		{#if step === 1}
			<div class="space-y-3">
				<p class="text-body-sm">Select and download a model for transcription.</p>
				<Button
					variant="secondary"
					onclick={async () => {
						modelSetupOpen = true;
						await modelStore.refresh();
					}}
				>
					Open model setup
				</Button>
				{#if modelReady}
					<p class="text-label-sm text-on-surface/70">Model setup complete.</p>
				{:else}
					<p class="text-label-sm text-on-surface/70">Choose a downloaded model to continue.</p>
				{/if}
			</div>
		{:else if step === 2}
			<div class="space-y-2">
				{#each permissions as permission (permission.kind)}
					{@const isOptional = permission.kind !== 'microphone'}
					<div
						class={`rounded-md border px-3 py-2.5 transition ${
							permission.granted
								? 'border-green/30 bg-green/5'
								: isOptional
									? 'border-surface-container bg-surface'
									: 'border-amber-500/30 bg-amber-500/5'
						}`}
					>
						<div class="flex items-center justify-between gap-3">
							<div class="flex items-center gap-2">
								{#if permission.granted}
									<svg class="size-4 shrink-0 text-green" viewBox="0 0 16 16" fill="currentColor">
										<path fill-rule="evenodd" d="M12.416 3.376a.75.75 0 0 1 .208 1.04l-5 7.5a.75.75 0 0 1-1.154.114l-3-3a.75.75 0 0 1 1.06-1.06l2.353 2.353 4.493-6.74a.75.75 0 0 1 1.04-.207Z" clip-rule="evenodd" />
									</svg>
								{:else if !isOptional}
									<span class="size-4 shrink-0 rounded-full border-2 border-amber-500/60 bg-amber-500/20"></span>
								{:else}
									<span class="size-4 shrink-0 rounded-full border-2 border-surface-container-high"></span>
								{/if}
								<div>
									<p class="text-body-sm capitalize">
										{permission.kind.replace(/_/g, ' ')}
										{#if isOptional}<span class="text-on-surface/40"> · optional</span>{/if}
									</p>
									{#if justGrantedByKind[permission.kind]}
										<p class="text-label-sm text-green">Just granted</p>
									{/if}
								</div>
							</div>
							{#if permission.granted}
								<span class="text-label-sm font-medium text-green">Granted</span>
							{:else if permission.can_request}
								<Button
									variant="secondary"
									disabled={requestingKind === permission.kind}
									onclick={() => grantPermission(permission.kind)}
								>
									{requestingKind === permission.kind ? 'Requesting…' : 'Grant permission'}
								</Button>
							{:else}
								<span class="text-label-sm text-on-surface/50">Unavailable</span>
							{/if}
						</div>
						{#if !permission.granted && permission.can_request && isOptional}
							<p class="mt-1.5 text-label-sm text-on-surface/40">
								{permission.kind === 'accessibility'
									? 'System Settings → Privacy & Security → Accessibility. Enable the toggle next to this app.'
									: 'System Settings → Privacy & Security → Input Monitoring. Enable the toggle next to this app.'}
							</p>
						{/if}
					</div>
				{/each}
			</div>
		{:else if step === 3}
			<div class="space-y-3">
				<PathSelectorField label="Output folder" bind:path={outputPath} onChange={saveOutputPath} />
				<p class="text-label-sm text-on-surface/70">Current path must exist or be creatable.</p>
			</div>
		{:else}
			<div class="space-y-3">
				<HotkeyCaptureField label="Open Scribe hotkey" bind:value={openHotkey} />
				<HotkeyCaptureField
					label="Dictate hotkey"
					bind:value={dictateHotkey}
					allowModifierOnly={true}
				/>
				<LabeledTextField label="Input label" bind:value={inputLabel} />
				<LabeledTextField label="Output label" bind:value={outputLabel} />
				<Button variant="secondary" onclick={saveHotkeysAndLabels}>Save hotkeys and labels</Button>
			</div>
		{/if}
		{#if gateError}
			<p class="mt-3 text-label-sm text-error">{gateError}</p>
		{/if}
		{#if saveError}
			<p class="mt-3 text-label-sm text-error">{saveError}</p>
		{/if}
	</section>

	<footer class="mt-auto flex items-center justify-between">
		<Button variant="secondary" onclick={refreshAllStatus}>
			{isRefreshing ? 'Refreshing...' : 'Refresh status'}
		</Button>
		<Button variant="primary" onclick={completeOnboarding}>
			Continue to Scribe
		</Button>
	</footer>
</div>

<ModelSetupModal
	open={modelSetupOpen}
	models={modelStore.models}
	progressByModel={modelStore.progressByModel}
	downloadingByModel={modelStore.downloadingByModel}
	statusByModel={modelStore.statusByModel}
	errorMessage={modelStore.error}
	canClose={canCloseModelSetup}
	onDownload={modelStore.download}
	onSelect={selectModel}
	onClose={closeModelSetup}
/>
