<script lang="ts">
	import {
		Home,
		FileText,
		Star,
		Upload,
		Settings,
	} from 'lucide-svelte';
	import SidebarNavItem from './SidebarNavItem.svelte';

	export type AppRoute =
		| 'home'
		| 'notes'
		| 'upload'
		| 'float'
		| 'settings';

	let {
		activeRoute,
		onnavigate,
	}: {
		activeRoute: AppRoute;
		onnavigate: (route: AppRoute) => void;
	} = $props();
</script>

<aside class="flex h-full min-h-0 w-44 shrink-0 flex-col border-r border-card bg-panel p-2.5">
	<div class="flex flex-1 flex-col gap-0.5">
		<SidebarNavItem
			label="Home"
			icon={Home}
			active={activeRoute === 'home'}
			onclick={() => onnavigate('home')}
		/>
		<SidebarNavItem
			label="Notes"
			icon={FileText}
			active={activeRoute === 'notes'}
			onclick={() => onnavigate('notes')}
		/>
		<SidebarNavItem
			label="Upload"
			icon={Upload}
			active={activeRoute === 'upload'}
			onclick={() => onnavigate('upload')}
		/>
		<SidebarNavItem label="Float" icon={Star} disabled badge="Coming soon" />
	</div>

	<div class="mt-3 border-t border-card pt-3">
		<SidebarNavItem
			label="Settings"
			icon={Settings}
			active={activeRoute === 'settings'}
			onclick={() => onnavigate('settings')}
		/>
		<p class="sf-meta-sm mt-3 px-2 text-fg-muted">local · no cloud</p>
	</div>
</aside>
