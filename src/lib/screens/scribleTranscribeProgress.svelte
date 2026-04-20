<script>
  import Button from "@lib/components/Button.svelte";
  import ProgressBar from "@lib/components/form/ProgressBar.svelte";
  //get selected models
  const progressSequence = [
		{ label: "Loading model name", complete: false },
		{ label: "Transcribing", complete: false },
		{ label: "Transcript saved", complete: false },
		{ label: "Clean up (if remove wav is on)", complete: false },
		{ label: "next model if selected", complete: false },
		{ label: "Result export", complete: false },
	];
</script>
<ProgressBar progress={40} sequence={progressSequence} sequenceMode="window" uiSize="lg" />

display FILENAME
display full path
Open Transcript {Button}secondary
Send to webhook {Button}secondary