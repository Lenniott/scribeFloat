<script lang="ts">
	import { onDestroy } from "svelte";
	import Button from "../Button.svelte";

	let {
		label,
		value = $bindable(""),
		placeholder = "Not set",
		allowModifierOnly = false,
	}: {
		label: string;
		value?: string;
		placeholder?: string;
		allowModifierOnly?: boolean;
	} = $props();

	let isCapturing = $state(false);
	let message = $state("");
	const fieldId = $derived(`hotkey-${label.toLowerCase().replace(/\s+/g, "-")}`);

	function isModifierKey(key: string): boolean {
		return key === "Meta" || key === "Control" || key === "Alt" || key === "Shift";
	}

	function normalizeKey(key: string): string {
		if (key.length === 1) return key.toUpperCase();
		if (/^F([1-9]|1[0-9]|2[0-4])$/.test(key)) return key;
		const map: Record<string, string> = {
			ArrowUp: "Up",
			ArrowDown: "Down",
			ArrowLeft: "Left",
			ArrowRight: "Right",
			Escape: "Esc",
			Enter: "Enter",
			Tab: "Tab",
			Backspace: "Backspace",
			Delete: "Delete",
			Home: "Home",
			End: "End",
			PageUp: "PageUp",
			PageDown: "PageDown",
			Insert: "Insert",
			" ": "Space",
		};
		return map[key] ?? key;
	}

	function formatHotkeyFromEvent(event: KeyboardEvent): string | null {
		const modifiers: string[] = [];
		if (event.metaKey || event.ctrlKey) modifiers.push("CmdOrCtrl");
		if (event.altKey) modifiers.push("Alt");
		if (event.shiftKey) modifiers.push("Shift");

		if (isModifierKey(event.key)) {
			if (!allowModifierOnly) return null;
			const modifierOnly = parseModifierFromKey(event.key);
			return modifierOnly ?? null;
		}
		const key = normalizeKey(event.key);
		if (!key || isModifierKey(key)) return null;
		return [...modifiers, key].join("+");
	}

	function parseModifierFromKey(key: string): string | null {
		if (key === "Meta") return "Command";
		if (key === "Control") return "Ctrl";
		if (key === "Alt") return "Alt";
		if (key === "Shift") return "Shift";
		return null;
	}

	function stopCapture() {
		isCapturing = false;
		window.removeEventListener("keydown", handleCapture);
	}

	function handleCapture(event: KeyboardEvent) {
		if (!isCapturing) return;
		event.preventDefault();
		const combo = formatHotkeyFromEvent(event);
		if (!combo) {
			message = "Include at least one non-modifier key.";
			return;
		}
		value = combo;
		message = `Captured: ${combo}`;
		stopCapture();
	}

	function startCapture() {
		message = "";
		if (isCapturing) return;
		isCapturing = true;
		window.addEventListener("keydown", handleCapture);
	}

	onDestroy(() => {
		if (isCapturing) {
			window.removeEventListener("keydown", handleCapture);
		}
	});
</script>

<div class="flex flex-col gap-1.5 text-left">
	<label class="text-label-sm font-semibold tracking-wide text-on-surface/80 uppercase" for={fieldId}
		>{label}</label
	>
	<div class="flex min-w-0 items-center gap-2">
		<input
			id={fieldId}
			type="text"
			readonly
			value={value || placeholder}
			class="text-label-md min-w-0 flex-1 truncate rounded-md bg-surface-container-lowest px-2 py-2 text-on-surface/90"
			title={value || placeholder}
		/>
		<Button variant="secondary" onclick={startCapture}>
			{isCapturing ? "Press keys..." : "Capture"}
		</Button>
	</div>
	{#if message}
		<p class="text-label-sm text-on-surface/70">{message}</p>
	{/if}
</div>
