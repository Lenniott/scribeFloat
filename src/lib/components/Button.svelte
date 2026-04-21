<script lang="ts">
	import type { IconProps } from "lucide-svelte";
	import type { Component, ComponentConstructorOptions, Snippet, SvelteComponent } from "svelte";

	type Variant = "primary" | "secondary" | "destructive" | "tertiary" | "normal";
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
		"inline-flex shrink-0 items-center justify-center gap-2 font-semibold transition-[opacity,background-color,color] disabled:pointer-events-none disabled:opacity-40";

	const variantClass: Record<Variant, string> = {
		primary:
			"bg-primary font-data text-on-primary hover:brightness-150",
		secondary: "border-primary border-1 text-primary hover:bg-primary hover:brightness-120 hover:text-on-primary",
		destructive:
			"bg-error-container text-on-error-container hover:brightness-150",
		tertiary:
			"bg-transparent text-primary hover:underline hover:decoration-primary hover:brightness-150 underline-offset-4",
		normal: "bg-transparent text-on-surface hover:bg-surface-container-high",
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
