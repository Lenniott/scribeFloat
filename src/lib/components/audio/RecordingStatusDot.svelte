<script lang="ts">
	type Status = "idle" | "recording" | "paused" | "error";

	let {
		status = "idle",
	}: {
		status?: Status;
	} = $props();

	const styles: Record<Status, string> = {
		idle: "bg-rim",
		recording: "bg-destructive animate-pulse",
		paused: "bg-rim",
		error: "bg-destructive",
	};
</script>

<span
	class="inline-block h-2 w-2 rounded-full {styles[status]}"
	title={status}
	aria-label={`Recording status: ${status}`}
></span>
