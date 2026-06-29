<script lang="ts">
	import { onMount } from 'svelte';
	import { browser } from '$app/environment';
	import { goto, beforeNavigate } from '$app/navigation';
	import { page } from '$app/state';
	import { invoke } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { watchThemeMode, type ThemeMode } from '@utils/theme';
	import '../app.css';

	import AppSidebar, { type AppRoute } from '@regions/AppSidebar.svelte';
	import ShellTitleBar from '@regions/TitleBar.svelte';
	import SettingsSidebar from '@regions/SettingsSidebar.svelte';
	import Toast from '@components/indicators/Toast.svelte';
	import Button from '@components/controls/Button.svelte';
	import Checkbox from '@components/controls/Checkbox.svelte';
	import Modal from '@primitives/layout/Modal.svelte';
	import DictateView from '@views/dictate.svelte';
	import OnboardingView from '@views/onboarding.svelte';
	import { appState } from '@stores/appState.svelte';
	import { loadNotes, executeDelete } from '@stores/appActions';
	import type { SettingsTab } from '@sections/settingsTypes';

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

	function isNoteEditorPath(pathname: string): boolean {
		const match = pathname.match(/^\/notes\/([^/]+)$/);
		return match !== null && match[1] !== 'new';
	}

	function runNoteLeaveGuard(proceed: () => void) {
		if (appState.noteLeaveGuard) {
			appState.noteLeaveGuard(
				() => {
					noteLeaveApproved = true;
					proceed();
				},
				() => {},
			);
			return;
		}
		proceed();
	}

	const viewParam = browser ? new URLSearchParams(window.location.search).get('view') : null;
	// Main shell uses ?view=history (Tauri); only dictate/onboarding are satellite windows.
	const isSatelliteWindow = viewParam === 'onboarding' || viewParam === 'dictate';

	let previousPath = $state('/');
	let noteLeaveApproved = false;

	const currentRoute = $derived(pathnameToRoute(page.url.pathname));
	const isSettingsRoute = $derived(page.url.pathname.startsWith('/settings'));
	const showNoteBack = $derived(isNoteEditorPath(page.url.pathname));

	const titleBarBack = $derived(
		showNoteBack ? () => void goto('/notes') :
		isSettingsRoute ? () => void goto(previousPath) :
		undefined
	);
	const titleBarBackLabel = $derived(
		showNoteBack ? 'Notes' :
		isSettingsRoute ? ROUTE_LABELS[pathnameToRoute(previousPath)] :
		'Back'
	);

	function guardedNavigate(next: AppRoute) {
		const path = ROUTE_PATHS[next];
		if (isNoteEditorPath(page.url.pathname)) {
			runNoteLeaveGuard(() => void goto(path));
			return;
		}
		void goto(path);
	}

	function openCapture() {
		void goto('/notes/new');
	}

	beforeNavigate(({ cancel, to }) => {
		if (
			to &&
			isNoteEditorPath(page.url.pathname) &&
			to.url.pathname !== page.url.pathname &&
			appState.noteLeaveGuard &&
			!noteLeaveApproved
		) {
			cancel();
			const dest = `${to.url.pathname}${to.url.search}`;
			appState.noteLeaveGuard(
				() => {
					noteLeaveApproved = true;
					void goto(dest).finally(() => {
						noteLeaveApproved = false;
					});
				},
				() => {},
			);
			return;
		}
		if (noteLeaveApproved) {
			noteLeaveApproved = false;
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
		<ShellTitleBar
			onNewNote={openCapture}
			onBack={titleBarBack}
			backLabel={titleBarBackLabel}
		/>
		<div class="flex min-h-0 flex-1 overflow-hidden">
			{#if isSettingsRoute}
				<SettingsSidebar
					activeTab={appState.settingsTab}
					ontabchange={(tab) => (appState.settingsTab = tab)}
				/>
			{:else}
				<AppSidebar activeRoute={currentRoute} onnavigate={guardedNavigate} />
			{/if}
			<main class="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-canvas">
				{@render children()}
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
				<Checkbox
					class="mr-auto"
					label="Don't ask again this session"
					bind:checked={appState.skipDeleteConfirm}
				/>
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
