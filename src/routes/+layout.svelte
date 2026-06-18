<script lang="ts">
	import { onMount } from 'svelte';
	import { browser } from '$app/environment';
	import { goto, beforeNavigate } from '$app/navigation';
	import { page } from '$app/state';
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { watchThemeMode, type ThemeMode } from '$lib/theme';
	import '../app.css';

	import AppSidebar, { type AppRoute } from '@lib/components/regions/AppSidebar.svelte';
	import ShellTitleBar from '@lib/components/regions/TitleBar.svelte';
	import SettingsSidebar from '@lib/components/regions/SettingsSidebar.svelte';
	import Toast from '@lib/components/ui/indicators/Toast.svelte';
	import Button from '@lib/components/ui/controls/Button.svelte';
	import Modal from '@lib/components/primitives/layout/Modal.svelte';
	import CaptureView from '@lib/views/capture.svelte';
	import DictateView from '@lib/views/dictate.svelte';
	import OnboardingView from '@lib/views/onboarding.svelte';
	import { appState } from '@lib/stores/appState.svelte';
	import { loadNotes, executeDelete } from '@lib/stores/appActions';
	import type { SettingsTab } from '@lib/components/sections/settingsTypes';

	let { children } = $props();

	type AppNavigateEvent = {
		route: AppRoute;
		settingsTab?: SettingsTab;
	};

	const ROUTE_PATHS: Record<AppRoute, string> = {
		home: '/',
		notes: '/notes',
		upload: '/upload',
		float: '/float',
		settings: '/settings',
	};

	const ROUTE_LABELS: Record<AppRoute, string> = {
		home: 'Home',
		notes: 'Notes',
		upload: 'Upload',
		float: 'Float',
		settings: 'Settings',
	};

	function pathnameToRoute(pathname: string): AppRoute {
		if (pathname.startsWith('/notes')) return 'notes';
		if (pathname.startsWith('/upload')) return 'upload';
		if (pathname.startsWith('/float')) return 'float';
		if (pathname.startsWith('/settings')) return 'settings';
		return 'home';
	}

	const viewParam = browser ? new URLSearchParams(window.location.search).get('view') : null;
	// Main shell uses ?view=history (Tauri); only dictate/onboarding are satellite windows.
	const isSatelliteWindow = viewParam === 'onboarding' || viewParam === 'dictate';

	let previousPath = $state('/');

	const currentRoute = $derived(pathnameToRoute(page.url.pathname));
	const isSettingsRoute = $derived(page.url.pathname.startsWith('/settings'));

	function guardedNavigate(next: AppRoute) {
		const path = ROUTE_PATHS[next];
		if (appState.captureOpen && appState.captureLeaveGuard) {
			appState.captureLeaveGuard(() => {
				appState.captureOpen = false;
				void goto(path);
			});
			return;
		}
		void goto(path);
	}

	function openCapture() {
		appState.captureVisitKey += 1;
		appState.captureOpen = true;
	}

	beforeNavigate(({ cancel, to }) => {
		if (appState.captureOpen && appState.captureLeaveGuard && to) {
			cancel();
			appState.captureLeaveGuard(() => {
				appState.captureOpen = false;
				void goto(to.url.pathname);
			});
		}
		if (to?.url.pathname.startsWith('/settings') && !page.url.pathname.startsWith('/settings')) {
			previousPath = page.url.pathname;
		}
	});

	onMount(() => {
		document.getElementById('sf-loading')?.remove();

		const trackSystemScheme = viewParam !== 'dictate';
		const localMode = (localStorage.getItem('sf_theme_mode') as ThemeMode | null) ?? 'system';
		let cleanup = watchThemeMode(localMode, { trackSystemScheme });
		let mounted = true;
		invoke<ThemeMode>('settings_get_theme_mode')
			.catch(() => 'system' as const)
			.then((themeMode) => {
				if (!mounted) return;
				cleanup();
				cleanup = watchThemeMode(themeMode, { trackSystemScheme });
			});

		function onStorage(e: StorageEvent) {
			if (e.key === 'sf_theme_mode' && e.newValue) {
				cleanup();
				cleanup = watchThemeMode(e.newValue as ThemeMode, { trackSystemScheme });
			}
		}
		window.addEventListener('storage', onStorage);

		if (isSatelliteWindow) {
			return () => {
				mounted = false;
				cleanup();
				window.removeEventListener('storage', onStorage);
			};
		}

		void loadNotes();
		const unlistenNoteP = listen('note://item-added', () => {
			void loadNotes();
		});
		const unlistenNavP = listen<AppNavigateEvent>('app://navigate', (event) => {
			const next = event.payload.route;
			if (next === 'settings' && event.payload.settingsTab) {
				appState.settingsTab = event.payload.settingsTab;
			}
			void goto(ROUTE_PATHS[next]);
		});

		return async () => {
			mounted = false;
			cleanup();
			window.removeEventListener('storage', onStorage);
			await (await unlistenNoteP)();
			await (await unlistenNavP)();
		};
	});
</script>

{#if viewParam === 'onboarding'}
	<OnboardingView />
{:else if viewParam === 'dictate'}
	<DictateView />
{:else}
	<div class="flex h-screen flex-col overflow-hidden bg-canvas">
		<ShellTitleBar onNewNote={openCapture} />
		<div class="flex min-h-0 flex-1 overflow-hidden">
			{#if isSettingsRoute}
				<SettingsSidebar
					activeTab={appState.settingsTab}
					ontabchange={(tab) => (appState.settingsTab = tab)}
					onback={() => void goto(previousPath)}
					backLabel={ROUTE_LABELS[pathnameToRoute(previousPath)]}
				/>
			{:else}
				<AppSidebar activeRoute={currentRoute} onnavigate={guardedNavigate} />
			{/if}
			<main class="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-canvas">
				{#if appState.captureOpen}
					<CaptureView
						visitKey={appState.captureVisitKey}
						onclose={() => (appState.captureOpen = false)}
						registerLeaveGuard={(handler) => {
							appState.captureLeaveGuard = handler;
						}}
					/>
				{:else}
					{@render children()}
				{/if}
			</main>
		</div>
	</div>

	<Modal
		open={appState.deleteTarget !== null}
		title="Delete recording?"
		description="This will permanently delete the transcript and any associated audio. This cannot be undone."
		maxWidthClass="max-w-sm"
		onClose={() => (appState.deleteTarget = null)}
	>
		{#snippet footer()}
			<div class="flex w-full items-center gap-4">
				<label class="mr-auto flex cursor-pointer items-center gap-2 sf-label-sm text-fg-dim">
					<input type="checkbox" bind:checked={appState.skipDeleteConfirm} />
					Don't ask again this session
				</label>
				<div class="flex gap-3">
					<Button
						variant="normal"
						disabled={appState.deleting}
						onclick={() => (appState.deleteTarget = null)}
					>
						Cancel
					</Button>
					<Button
						variant="destructive"
						disabled={appState.deleting}
						onclick={() => {
							const target = appState.deleteTarget!;
							appState.deleteTarget = null;
							void executeDelete(target);
						}}
					>
						{appState.deleting ? 'Deleting…' : 'Delete'}
					</Button>
				</div>
			</div>
		{/snippet}
	</Modal>

	<Toast message={appState.toastMessage} state={appState.toastState} position="bottom-center" />
{/if}
