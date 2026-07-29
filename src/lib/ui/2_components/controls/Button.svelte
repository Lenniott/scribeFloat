<script lang="ts">
	import type { IconProps } from "lucide-svelte";
	import type { Component, ComponentConstructorOptions, Snippet, SvelteComponent } from "svelte";

	type Variant = "primary" | "destructive" | "ghost" | "normal" | "active";
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
		title,
		"aria-label": ariaLabel,
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
		title?: string;
		"aria-label"?: string;
	} = $props();

	const base =
		"inline-flex shrink-0 cursor-pointer leading-none items-center justify-center gap-2 transition-[opacity,background-color,color] disabled:pointer-events-none disabled:opacity-40 sf-focus-ring";

	const variantClass: Record<Variant, string> = {
		primary: "bg-brand text-on-brand hover:bg-brand-hover hover:text-on-brand-hover",
		destructive: "bg-destructive text-on-destructive hover:bg-destructive-hover",
		ghost: "bg-transparent text-fg hover:bg-card",
		normal: "bg-transparent border border-rim text-fg hover:bg-fill",
		active: "bg-rim text-fg hover:brightness-105",
	};

	const sizeLayout: Record<
		Size,
		{ padWithIcon: string; padPlain: string; text: string; icon: string }
	> = {
		normal: {
			padWithIcon: "pl-3.5 pr-4 h-10",
			padPlain: "px-4 h-9",
			text: "sf-label-md",
			icon: "size-4",
		},
		small: {
			padWithIcon: "px-2.5 h-7",
			padPlain: "px-2.5 h-7",
			text: "sf-label-sm",
			icon: "size-3.5",
		},
	};

	let padClass = $derived(Icon ? sizeLayout[size].padWithIcon : sizeLayout[size].padPlain);

	let classes = $derived(
		[
			base,
			variantClass[variant],
			`${padClass} ${sizeLayout[size].text} rounded-md`,
			className,
		]
			.filter(Boolean)
			.join(" "),
	);

	let iconClass = $derived(sizeLayout[size].icon);
</script>

<button
	class={classes}
	{type}
	{disabled}
	{title}
	aria-label={ariaLabel}
	onclick={onclick}
>
	{#if Icon}
		<Icon class={iconClass} strokeWidth={2} />
	{/if}
	{@render children?.()}
</button>
