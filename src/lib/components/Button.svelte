<script lang="ts">
	import type { IconProps } from "lucide-svelte";
	import type { Component, ComponentConstructorOptions, Snippet, SvelteComponent } from "svelte";

	type Variant = "primary" | "secondary" | "destructive" | "transparent" | "normal" | "active";
	type Size = "normal" | "small";

	/** lucide-svelte still types icons as class components; `Component` covers Svelte 5 function components only */
	type LucideIcon =
		| Component<IconProps>
		| (new (options: ComponentConstructorOptions<IconProps>) => SvelteComponent<IconProps>);

	let {
		variant = "normal",
		size = "normal",
		icon: Icon,
		type = "button",
		disabled = false,
		class: className = "",
		children,
		onclick,
	}: {
		variant?: Variant;
		size?: Size;
		/** Lucide icon component from `lucide-svelte/icons/...` */
		icon?: LucideIcon;
		type?: "button" | "submit" | "reset";
		disabled?: boolean;
		class?: string;
		children?: Snippet;
		onclick?: (e: MouseEvent) => void;
	} = $props();

	const base =
		"inline-flex shrink-0 cursor-pointer items-center justify-center gap-2 font-normal tracking-wide transition-[opacity,background-color,color] disabled:pointer-events-none disabled:opacity-40";

	const variantClass: Record<Variant, string> = {
		primary: "bg-primary text-on-primary hover:brightness-110",
		secondary: "border-secondary border-1 text-secondary hover:bg-secondary hover:brightness-120 hover:text-on-secondary",
		destructive: "bg-error-container text-on-error-container hover:brightness-150",
		transparent: "bg-transparent text-on-surface hover:bg-surface-container-high",
		normal: "bg-transparent border border-surface-container-highest text-on-surface hover:bg-surface-container-high",
		active: "bg-active text-on-active hover:brightness-95",
	};

	const sizeClass: Record<Size, { pad: string; text: string; icon: string }> = {
		normal: {
			pad: "px-4 py-2",
			text: "text-label-md",
			icon: "size-4",
		},
		small: {
			pad: "px-2.5 py-1.5",
			text: "text-label-sm",
			icon: "size-3.5",
		},
	};

	let classes = $derived(
		[
			base,
			variantClass[variant],
			`${sizeClass[size].pad} ${sizeClass[size].text} rounded-md`,
			className,
		]
			.filter(Boolean)
			.join(" "),
	);

	let iconClass = $derived(sizeClass[size].icon);
</script>

<button
	class={classes}
	{type}
	{disabled}
	onclick={onclick}
>
	{#if Icon}
		<Icon class={iconClass} strokeWidth={2} />
	{/if}
	{@render children?.()}
</button>
