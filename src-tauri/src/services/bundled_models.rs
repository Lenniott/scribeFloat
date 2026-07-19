//! Seed and integrity helpers for models shipped inside the signed app bundle.
//!
//! Runtime never downloads models. When the writable copy under `{app_data}/models`
//! is missing, empty, or fails its SHA-256 pin, we re-copy from the installed
//! app's resource directory (offline self-heal), then re-check the pin.

use std::path::Path;

/// Compute the lowercase-hex SHA-256 of a file on disk, streaming so large
/// models never land in memory all at once.
pub fn file_sha256_hex(path: &Path) -> std::io::Result<String> {
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    Ok(hex)
}

/// True when dest is missing, empty, unreadable, or its hash does not match.
pub fn dest_needs_bundle_restore(dest: &Path, expected_sha: &str) -> bool {
    match std::fs::metadata(dest) {
        Ok(m) if m.is_file() && m.len() > 0 => match file_sha256_hex(dest) {
            Ok(actual) => actual != expected_sha.to_ascii_lowercase(),
            Err(_) => true,
        },
        _ => true,
    }
}

/// Copy `resource_dir/file_name` → `dest` when the resource is a real non-empty
/// file (dev builds ship 0-byte placeholders — those must not overwrite).
pub fn try_copy_from_bundle(resource_dir: &Path, file_name: &str, dest: &Path) -> bool {
    let bundled = resource_dir.join(file_name);
    let has_content = std::fs::metadata(&bundled)
        .map(|m| m.is_file() && m.len() > 0)
        .unwrap_or(false);
    if !has_content {
        return false;
    }
    if let Some(parent) = dest.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::copy(&bundled, dest) {
        Ok(_) => true,
        Err(e) => {
            tracing::warn!(
                error = %e,
                bundled = %bundled.display(),
                dest = %dest.display(),
                "could not restore bundled model from app resources"
            );
            false
        }
    }
}

/// If `dest` fails integrity, try one offline restore from `resource_dir`, then
/// re-check. Returns whether `dest` is trusted afterward.
pub fn ensure_bundled_file(
    resource_dir: Option<&Path>,
    dest: &Path,
    file_name: &str,
    expected_sha: &str,
) -> bool {
    if !dest_needs_bundle_restore(dest, expected_sha) {
        return true;
    }
    let Some(resource_dir) = resource_dir else {
        return false;
    };
    if !try_copy_from_bundle(resource_dir, file_name, dest) {
        return !dest_needs_bundle_restore(dest, expected_sha);
    }
    let ok = !dest_needs_bundle_restore(dest, expected_sha);
    if ok {
        tracing::info!(
            file = file_name,
            dest = %dest.display(),
            "restored bundled model from app resources"
        );
    }
    ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn file_sha256_hex_matches_known_vector() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.bin");
        std::fs::write(&path, b"abc").unwrap();
        assert_eq!(
            file_sha256_hex(&path).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn dest_needs_restore_when_missing_or_empty() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("m.bin");
        assert!(dest_needs_bundle_restore(&dest, "deadbeef"));
        std::fs::write(&dest, b"").unwrap();
        assert!(dest_needs_bundle_restore(&dest, "deadbeef"));
    }

    #[test]
    fn dest_needs_restore_on_hash_mismatch_only() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("m.bin");
        std::fs::write(&dest, b"abc").unwrap();
        let good = file_sha256_hex(&dest).unwrap();
        assert!(!dest_needs_bundle_restore(&dest, &good));
        assert!(dest_needs_bundle_restore(&dest, "0".repeat(64).as_str()));
    }

    #[test]
    fn ensure_restores_bad_dest_from_bundle() {
        let dir = tempfile::tempdir().unwrap();
        let resource = dir.path().join("resources");
        let models = dir.path().join("models");
        std::fs::create_dir_all(&resource).unwrap();
        std::fs::create_dir_all(&models).unwrap();

        let mut good = std::fs::File::create(resource.join("m.bin")).unwrap();
        good.write_all(b"trusted-bytes").unwrap();
        drop(good);
        let sha = file_sha256_hex(&resource.join("m.bin")).unwrap();

        let dest = models.join("m.bin");
        std::fs::write(&dest, b"corrupt").unwrap();
        assert!(dest_needs_bundle_restore(&dest, &sha));

        assert!(ensure_bundled_file(
            Some(resource.as_path()),
            &dest,
            "m.bin",
            &sha
        ));
        assert_eq!(std::fs::read(&dest).unwrap(), b"trusted-bytes");
    }

    #[test]
    fn ensure_skips_zero_byte_bundle_placeholder() {
        let dir = tempfile::tempdir().unwrap();
        let resource = dir.path().join("resources");
        let models = dir.path().join("models");
        std::fs::create_dir_all(&resource).unwrap();
        std::fs::create_dir_all(&models).unwrap();
        std::fs::write(resource.join("m.bin"), b"").unwrap();

        let dest = models.join("m.bin");
        std::fs::write(&dest, b"corrupt").unwrap();
        let sha = "0".repeat(64);
        assert!(!ensure_bundled_file(
            Some(resource.as_path()),
            &dest,
            "m.bin",
            &sha
        ));
        assert_eq!(std::fs::read(&dest).unwrap(), b"corrupt");
    }
}
