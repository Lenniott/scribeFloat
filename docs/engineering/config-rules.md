# Config Rules

> Load this when adding or modifying fields in `Config` (`types.rs`).

---

- All config fields must have `#[serde(default)]` or `#[serde(default = "fn")]` so old config files deserialise cleanly after a schema change.
- If a new field changes existing behaviour, document the default clearly in the field's doc comment.
- `ConfigService::update()` saves atomically (temp file + rename). Always use it — never write `config.json` directly.
- After changing `Config`, verify a config file missing the new field still loads without panicking.
