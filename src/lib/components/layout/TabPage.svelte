<script lang="ts">
	import type { Snippet } from "svelte";

	export type TabPageItem = {
		id: string;
		label: string;
		disabled?: boolean;
	};

	type Mode = "panel" | "section";

	let {
		tabs,
		activeId = $bindable<string | undefined>(),
		mode = "panel",
		class: className = "",
		children,
	}: {
		tabs: TabPageItem[];
		activeId?: string;
		mode?: Mode;
		class?: string;
		children?: Snippet<[TabPageItem | undefined]>;
	} = $props();

	$effect(() => {
		if (tabs.length === 0) return;
		let activeExists = false;
		let firstEnabledId: string | undefined;
		for (const tab of tabs) {
			if (!tab.disabled && firstEnabledId === undefined) {
				firstEnabledId = tab.id;
			}
			if (tab.id === activeId && !tab.disabled) {
				activeExists = true;
				break;
			}
		}
		if (!activeExists) {
			activeId = firstEnabledId;
		}
	});

	let activeTab = $derived(tabs.find((tab) => tab.id === activeId));

	const wrapperClass: Record<Mode, string> = {
		panel: "rounded-md bg-card shadow-ambient",
		section: "rounded-md border border-card/60 bg-card/50",
	};

	const tabListClass: Record<Mode, string> = {
		panel:
			"flex items-center gap-1 overflow-x-auto border-b border-card/60 bg-panel p-2",
		section:
			"flex items-center gap-1 overflow-x-auto border-b border-card/60 bg-panel/70 p-1.5",
	};
	const tabButtonClass: Record<Mode, string> = {
		panel: "px-3 py-2 sf-label-md",
		section: "px-2.5 py-1.5 sf-label-sm",
	};
	const contentClass: Record<Mode, string> = {
		panel: "p-4",
		section: "p-3",
	};
</script>

<!--
  Tabbed pane primitive. For bounded app panes, wrap the parent in
  `flex h-full min-h-0 flex-col overflow-hidden`, keep the tab list as shrink-0
  chrome, and place tab content inside ScrollablePanel.
-->
<section class="{wrapperClass[mode]} {className}">
	<div class="{tabListClass[mode]} shrink-0" role="tablist" aria-label="Tabs">
		{#each tabs as tab (tab.id)}
			<button
				type="button"
				role="tab"
				aria-selected={activeId === tab.id}
				aria-controls={`tab-panel-${tab.id}`}
				disabled={tab.disabled}
				class="{tabButtonClass[mode]} whitespace-nowrap transition-colors disabled:opacity-40 {activeId === tab.id
					? 'border-0 border-b-2 border-active bg-active/15 text-fg'
					: 'border-0 border-b-2 border-transparent text-fg-dim hover:bg-fill hover:text-fg'}"
				onclick={() => (activeId = tab.id)}
			>
				{tab.label}
			</button>
		{/each}
	</div>

	<div id={`tab-panel-${activeTab?.id ?? "empty"}`} role="tabpanel" class={contentClass[mode]}>
		{@render children?.(activeTab)}
	</div>
</section>
