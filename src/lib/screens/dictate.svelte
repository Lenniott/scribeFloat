<script>
  import AudioWaveFormVisualizer from "@lib/components/audio/AudioWaveFormVisualizer.svelte";
  import RecordingStatusDot from "@lib/components/audio/RecordingStatusDot.svelte";
  import RecordingTimer from "@lib/components/audio/RecordingTimer.svelte";
  import IconButton from "@lib/components/IconButton.svelte";
  import { X as Close } from "lucide-svelte";
  //not hooked up to real things
</script>



<div class="flex gap-2 items-center">
    <div class="flex items-center gap-1.5 mr-4">
        <RecordingStatusDot status="recording" />
        <RecordingTimer elapsedSeconds={94} />
    </div>
    <AudioWaveFormVisualizer
        micLevel={0.55}
        speakerLevel={0.35}
        speakerEnabled={false}
        size="small"
    />
    <IconButton variant="normal" icon={Close} aria-label="Close" />
</div>