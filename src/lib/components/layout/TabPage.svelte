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
		panel: "rounded-md bg-surface-container-low shadow-ambient",
		section: "rounded-md border border-outline-variant/60 bg-surface-container-low/50",
	};

	const tabListClass: Record<Mode, string> = {
		panel:
			"flex items-center gap-1 overflow-x-auto border-b border-outline-variant/60 bg-surface-container-lowest p-2",
		section:
			"flex items-center gap-1 overflow-x-auto border-b border-outline-variant/60 bg-surface-container-lowest/70 p-1.5",
	};
	const tabButtonClass: Record<Mode, string> = {
		panel: "px-3 py-2 text-label-md",
		section: "px-2.5 py-1.5 text-label-sm",
	};
	const contentClass: Record<Mode, string> = {
		panel: "p-4",
		section: "p-3",
	};
</script>

<section class="{wrapperClass[mode]} {className}">
	<div class={tabListClass[mode]} role="tablist" aria-label="Tabs">
		{#each tabs as tab (tab.id)}
			<button
				type="button"
				role="tab"
				aria-selected={activeId === tab.id}
				aria-controls={`tab-panel-${tab.id}`}
				disabled={tab.disabled}
				class="{tabButtonClass[mode]} rounded-sm font-semibold tracking-wide whitespace-nowrap uppercase transition-colors disabled:opacity-40 {activeId === tab.id
					? 'bg-tertiary text-on-primary'
					: 'text-on-surface/70 hover:bg-surface-container-high hover:text-on-surface'}"
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
