---
id: "0067"
title: Export transcript as markdown file from note view
status: active
---

# Export transcript as markdown file from note view

As a user reading a finished transcript in the note editor, I want to export it as a `.md` file to my filesystem so that I can share it, archive it, or open it in another tool without having to copy-paste.

Currently the only way to get the text out is to select-all and copy. There is no export action in the note editor UI.

## Notes

- The Tauri `dialog` plugin (`save` dialog) should be used to let the user pick a save path — avoids hardcoding a location.
- The export should write the same markdown the editor contains (the `content` field of the note), not a re-serialised version.
- A logical place for the trigger is a secondary action in the note editor header row (next to the RecordingStrip), or an overflow menu if one is added.
- Consider whether the "keep md file" setting (which already writes a sidecar `.md` to a fixed path) overlaps with this — it does not replace on-demand export since the sidecar path is internal and not user-chosen.
