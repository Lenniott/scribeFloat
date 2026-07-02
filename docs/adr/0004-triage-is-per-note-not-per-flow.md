# Triage is per-Note, not per-Flow-run

When Float runs on a Note, the user reviews and approves the results once — for the Note as a whole — not once per Flow or per Step. If multiple Flows run on a Note before the user triages it, their Results merge into one Triage item. Once a Note has been triaged, subsequent Float runs on it are applied directly without a second Triage cycle.

The alternative — per-Flow-run Triage — was rejected because it would require the user to triage the same Note multiple times as Flows accumulate results. The cost of reviewing is proportional to the number of Flows, not the number of Notes. The user's mental model is "I'm reviewing this Note" not "I'm reviewing this Flow's output on this Note."

**Consequence:** The data model must track Triage status at the Note level, not the Result level. A Result's `draft/edited/approved` status is an internal Float concept; Triage is the user-facing surface over the Note's pending state.
