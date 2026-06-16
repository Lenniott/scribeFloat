<script lang="ts">
	import type { IconProps } from "lucide-svelte";
	import type { Component, ComponentConstructorOptions, SvelteComponent } from "svelte";

	type Variant = "primary" | "destructive" | "normal";
	type Size = "normal" | "small";

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
		iconExtraClass = "",
		onclick,
		"aria-label": ariaLabel,
	}: {
		variant?: Variant;
		size?: Size;
		icon: LucideIcon;
		type?: "button" | "submit" | "reset";
		disabled?: boolean;
		class?: string;
		iconExtraClass?: string;
		onclick?: (e: MouseEvent) => void;
		"aria-label": string;
	} = $props();

	const base =
		"inline-flex shrink-0 items-center justify-center rounded-md cursor-pointer transition-[opacity,background-color,color] disabled:pointer-events-none disabled:opacity-40";

	const variantClass: Record<Variant, string> = {
		primary: "bg-brand text-on-brand hover:bg-brand-hover hover:text-on-brand-hover",
		destructive: "bg-transparent text-destructive hover:bg-destructive hover:text-on-destructive",
		normal: "bg-transparent text-fg hover:bg-card",
	};

	const sizeClass: Record<Size, { button: string; icon: string }> = {
		normal: {
			button: "size-10 p-0",
			icon: "size-4",
		},
		small: {
			button: "size-8 p-0",
			icon: "size-3.5",
		},
	};

	let classes = $derived([base, variantClass[variant], sizeClass[size].button, className].filter(Boolean).join(" "));
	let iconClass = $derived([sizeClass[size].icon, iconExtraClass].filter(Boolean).join(" "));
</script>

<button
	class={classes}
	{type}
	{disabled}
	onclick={onclick}
	aria-label={ariaLabel}
	title={ariaLabel}
>
	<Icon class={iconClass} strokeWidth={2} />
</button>
