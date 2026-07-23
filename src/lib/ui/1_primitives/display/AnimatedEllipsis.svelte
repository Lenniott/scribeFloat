<script lang="ts">
	/**
	 * Animated trailing dots for indeterminate waits ("Loading model…").
	 * Inherits the surrounding text's font and color; the label itself stays
	 * in the caller so screen readers get real text while the dots are
	 * decoration.
	 */
	let { class: className = "" }: { class?: string } = $props();
</script>

<span class={`animated-ellipsis ${className}`.trim()} aria-hidden="true"
	><span>.</span><span>.</span><span>.</span></span
>

<style>
	.animated-ellipsis span {
		opacity: 0.2;
		animation: ellipsis-pulse 1.2s infinite;
	}

	.animated-ellipsis span:nth-child(2) {
		animation-delay: 0.2s;
	}

	.animated-ellipsis span:nth-child(3) {
		animation-delay: 0.4s;
	}

	@keyframes ellipsis-pulse {
		0%,
		60%,
		100% {
			opacity: 0.2;
		}
		20%,
		40% {
			opacity: 1;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.animated-ellipsis span {
			animation: none;
			opacity: 1;
		}
	}
</style>
