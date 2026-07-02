# Hydration prompt test runner

Automates the manual §9.1 validation from
[`docs/explorations/active/2026-07-02-hydration-prompt-test-kit.md`](../../../docs/explorations/active/2026-07-02-hydration-prompt-test-kit.md):
runs the extract → resolve prompt pipeline over five fixed test chunks against a local
Ollama, scores the results against expected labels, and writes one shareable text report.

**No venv, no pip.** The script uses only the Python standard library, so the Python that
ships with macOS is enough. (A venv is only ever needed to isolate third-party `pip`
packages — plain-stdlib scripts run anywhere.)

## Run it

```bash
cd tools/project_package/hydration_test

python3 run_test.py                       # defaults: gemma3:270m, temp 0, 1 run
python3 run_test.py --temps 0,0.4,0.8     # temperature sweep
python3 run_test.py --runs 3              # repeat each temp 3x (variance check)
python3 run_test.py --model qwen3.5:4b    # try a different local model
```

Ollama must be running (`ollama serve` or the desktop app) and the model pulled
(`ollama pull gemma3:270m`).

## Output

One file per invocation in `results/`, e.g. `results/hydration_gemma3_270m_20260702-181530.txt`.
It contains, per temperature × run × chunk:

- the raw model response for both prompts (so failures can be diagnosed, not just counted)
- the parsed phrase list, with extraction-quality flags (`PARAPHRASE`, `CLAUSE?`, `OVERLAP`)
- per-check scoring: `PASS` / `MISMATCH` / `NOT_EXTRACTED`, with borderline rows logged as
  `NOTE` rather than scored
- a totals summary at the end

Share the whole file — the raw responses are the useful part.

## What the chunks test

| Chunk | Tests |
|---|---|
| 1 | In-sentence definition: "the nightly sync job" must come back `text`; "CRM export"/"reporting database" are over-flag detectors |
| 2 | Fully self-contained control: zero `missing` expected ("last sprint" recorded as a borderline note, not scored) |
| 3 | Genuine compression: "the usual pipeline" and "Acme" must come back `missing`; the pipeline should later match chunk 1's defines |
| 4 | Undefined acronym: "the QBR" must come back `missing` |
| 5 | Telegraphic written self-note (hardest case): "D" and "last time" must come back `missing` |

Prompt or chunk changes belong in the test kit doc first; this script embeds the current
revisions (extract rev 3, resolve rev 2) and should be kept in sync with it.
