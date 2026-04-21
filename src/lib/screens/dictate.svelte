<script>
  import CircularAudioVisualizer from "@lib/components/audio/CircularAudioVisualizer.svelte";
  import RecordingStatusDot from "@lib/components/audio/RecordingStatusDot.svelte";
  import RecordingTimer from "@lib/components/audio/RecordingTimer.svelte";
  import IconButton from "@lib/components/IconButton.svelte";
  import { X as Close } from "lucide-svelte";
  //not hooked up to real things
</script>



<div class="flex gap-2 items-center">
    <CircularAudioVisualizer
        size={120}
        micLevel={0.55}
        speakerLevel={0.35}
        innerBaseScale={0.25}
        ampInner={0.12}
        outerScale={1.2}
        ampOuter={0.13}
        speakerEnabled={false}
        showLegend={false}
    >
        {#snippet children()}
            <div class="flex items-center gap-1.5">
                <RecordingStatusDot status="recording" />
                <RecordingTimer elapsedSeconds={94} />
            </div>
        {/snippet}
    </CircularAudioVisualizer>
    <div class="flex w-14 justify-end">
    <IconButton variant="normal" icon={Close} aria-label="Close" />
    </div>
</div>