<script lang="ts">
	type Status = "idle" | "recording" | "paused" | "error";

	let {
		status = "idle",
	}: {
		status?: Status;
	} = $props();

	const styles: Record<Status, string> = {
		idle: "bg-surface-container-highest",
		recording: "bg-error-container brightness-200 animate-pulse",
		paused: "bg-active",
		error: "bg-primary",
	};
</script>

<span
	class="inline-block h-2 w-2 rounded-sm {styles[status]}"
	title={status}
	aria-label={`Recording status: ${status}`}
></span>
