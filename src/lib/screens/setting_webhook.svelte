<script>
  import LabeledTextField from "@lib/components/form/LabeledTextField.svelte";
import ToggleSwitch from "@lib/components/form/ToggleSwitch.svelte";

</script>
WEBHOOK URL
https://n8n.lenniott.uk/webhook/ob-doc
POST transcripts and dictations to this URL when
processing finishes.
AUTH HEADER NAME
x-ob-key
AUTH HEADER VALUE
••• .{LabeledTextField}

AUTO-SEND SCRIBE & TRANSCRIBE
OFFOFF{ToggleSwitch}
When ON, Scribe and Transcribe automatically POST the
md file on completion. When OFF, a Send button appears
on the done screen.
AUTO-SEND DICTATE
OFF{ToggleSwitch}
When ON, Dictate automatically POSTs the transcribed
text to the webhook on completion.

PAYLOAD SHAPES

Dictate sends JSON:
---------------
{ "source": "dictate", "text": "..", "word_coun
t": 12, "duration _seconds": 4.2, "timestamp": "2
026-01-01T12:00:00" }
------------------

Scribe & Transcribe send multipart/form-data:
-------------------
source: "scribe"
file: transcript.md
word _count: 84
duration_seconds: 62.3
model: base
created_at: 2026-01-01T12:00:00
-----------------------