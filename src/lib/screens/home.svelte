<script lang="ts">
	import { onMount } from 'svelte';
	import { listen } from '@tauri-apps/api/event';
	import StatTile from '@lib/components/ui/indicators/StatTile.svelte';
	import RecentNoteCard from '@lib/components/ui/cards/RecentNoteCard.svelte';
	import {
		fetchDashboardStats,
		type DashboardStats,
		type HistoryListItem,
	} from '@lib/services/historyActions';
	import { formatWeekDuration } from '@lib/services/historyFormat';

	let {
		items,
		loading,
		onselect,
		onseeall,
	}: {
		items: HistoryListItem[];
		loading: boolean;
		onselect: (item: HistoryListItem) => void;
		onseeall: () => void;
	} = $props();

	let stats = $state<DashboardStats | null>(null);

	const recentItems = $derived(items.slice(0, 6));
	const todayLabel = $derived(
		new Date().toLocaleDateString(undefined, {
			weekday: 'long',
			day: 'numeric',
			month: 'long',
			year: 'numeric',
		}),
	);

	onMount(() => {
		void fetchDashboardStats().then((s) => (stats = s));
		const unlistenP = listen('note://item-added', () => {
			void fetchDashboardStats().then((s) => (stats = s));
		});
		return async () => (await unlistenP)();
	});
</script>

<div class="flex h-full flex-col overflow-y-auto p-6">
	<header class="mb-6">
		<h1 class="sf-headline-sm text-fg">Home</h1>
		<p class="mt-1 sf-body-md text-fg-dim">{todayLabel}</p>
	</header>

	<div class="mb-7 grid grid-cols-4 gap-3">
		<StatTile value={stats ? String(stats.transcript_count) : '—'} label="Notes" />
		<StatTile value="—" label="Float layers" />
		<StatTile value="—" label="Drafts to review" />
		<StatTile
			value={formatWeekDuration(stats?.recorded_this_week_secs)}
			label="Recorded this week"
		/>
	</div>

	<div class="mb-3 flex items-center justify-between">
		<h2 class="sf-label-md text-fg">Recent</h2>
		<button type="button" class="sf-label-md text-brand hover:text-brand-hover" onclick={onseeall}>
			See all →
		</button>
	</div>

	{#if loading}
		<p class="sf-body-md text-fg-muted">Loading…</p>
	{:else if recentItems.length === 0}
		<p class="sf-body-md text-fg-muted">No notes yet.</p>
	{:else}
		<div class="flex flex-col gap-2.5">
			{#each recentItems as item (item.id)}
				<RecentNoteCard {item} onselect={() => onselect(item)} />
			{/each}
		</div>
	{/if}
</div>
