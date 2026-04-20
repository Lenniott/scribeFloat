<script>
  import ProgressBar from "@lib/components/form/ProgressBar.svelte";
  const progressSequence = [
		{ label: "Transcribing...", complete: false },
		{ label: "Text copied", complete: false },
	];
    //slight delay on closing panel like half a second
</script>
<ProgressBar progress={100} sequence={progressSequence} sequenceMode="current" uiSize="sm" />