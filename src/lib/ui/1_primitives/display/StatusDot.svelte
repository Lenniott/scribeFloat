<script lang="ts">
	type Status = "idle" | "recording" | "paused" | "error";

	let {
		status = "idle",
		/** Opacity pulse while recording; disable on transparent HUDs (e.g. dictate) where it reads as flicker. */
		pulseWhileRecording = true,
	}: {
		status?: Status;
		pulseWhileRecording?: boolean;
	} = $props();

	const styles: Record<Status, string> = {
		idle: "bg-rim",
		recording: "bg-destructive",
		paused: "bg-rim",
		error: "bg-destructive",
	};

	const pulseClass = $derived(
		pulseWhileRecording && status === "recording" ? "animate-pulse" : "",
	);
</script>

<span
	class="inline-block h-2 w-2 rounded-full {styles[status]} {pulseClass}"
	title={status}
	aria-label={`Recording status: ${status}`}
></span>
