<script lang="ts">
	type Status = "idle" | "recording" | "paused" | "error";

	let {
		status = "idle",
	}: {
		status?: Status;
	} = $props();

	const styles: Record<Status, string> = {
		idle: "bg-surface-highest",
		recording: "bg-error animate-pulse",
		paused: "bg-surface-highest",
		error: "bg-error",
	};
</script>

<span
	class="inline-block h-2 w-2 rounded-full {styles[status]}"
	title={status}
	aria-label={`Recording status: ${status}`}
></span>
