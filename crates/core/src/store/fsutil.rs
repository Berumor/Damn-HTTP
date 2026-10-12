//! Small filesystem helpers.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

/// The line the app keeps in a workspace's `.gitignore` (D-016).
pub const GITIGNORE_ENTRY: &str = "/.damnhttp/";

/// Writes `contents` to `path` through a temporary file in the same
/// directory, so a crash never leaves a half-written file. Does nothing if
/// the file already has exactly these contents.
pub fn write_atomic(path: &Path, contents: &str) -> io::Result<()> {
    if let Ok(existing) = fs::read(path)
        && existing == contents.as_bytes()
    {
        return Ok(());
    }
    let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path has no file name",
        ));
    };
    // Starts with a dot and does not end in `.yaml`, so the tree never
    // mistakes a leftover for a request.
    let temporary = path.with_file_name(format!(".{file_name}.tmp"));
    let result = (|| {
        let mut file = fs::File::create(&temporary)?;
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        // Best effort: the original error is the one worth reporting.
        let _ = fs::remove_file(&temporary);
    }
    result
}

/// Returns the `.gitignore` text with the local-state directory ignored, or
/// `None` if `existing` already ignores it.
pub fn gitignore_with_entry(existing: &str) -> Option<String> {
    let mut ignored = false;
    for line in existing.lines() {
        let line = line.trim();
        match line.strip_prefix('!') {
            Some(negated) if is_local_dir_pattern(negated) => ignored = false,
            None if is_local_dir_pattern(line) => ignored = true,
            _ => {}
        }
    }
    if ignored {
        return None;
    }
    let mut updated = existing.to_owned();
    if !updated.is_empty() && !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(GITIGNORE_ENTRY);
    updated.push('\n');
    Some(updated)
}

fn is_local_dir_pattern(pattern: &str) -> bool {
    matches!(
        pattern,
        "/.damnhttp/" | ".damnhttp/" | "/.damnhttp" | ".damnhttp"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_entry_is_added_when_missing() {
        assert_eq!(gitignore_with_entry("").as_deref(), Some("/.damnhttp/\n"));
        assert_eq!(
            gitignore_with_entry("node_modules/\n").as_deref(),
            Some("node_modules/\n/.damnhttp/\n")
        );
        assert_eq!(
            gitignore_with_entry("no-newline").as_deref(),
            Some("no-newline\n/.damnhttp/\n")
        );
    }

    #[test]
    fn an_existing_entry_is_left_alone() {
        for existing in [
            "/.damnhttp/\n",
            ".damnhttp\n",
            "target\n  .damnhttp/  \nother\n",
            "/.damnhttp\r\n",
        ] {
            assert_eq!(gitignore_with_entry(existing), None, "{existing:?}");
        }
    }

    #[test]
    fn a_later_negation_is_overridden_again() {
        assert_eq!(
            gitignore_with_entry(".damnhttp/\n!.damnhttp/\n").as_deref(),
            Some(".damnhttp/\n!.damnhttp/\n/.damnhttp/\n")
        );
        assert_eq!(gitignore_with_entry("!.damnhttp/\n.damnhttp/\n"), None);
    }

    #[test]
    fn similar_lines_do_not_count() {
        for existing in [
            "# .damnhttp/\n",
            ".damnhttp/values.yaml\n",
            "x/.damnhttp/\n",
        ] {
            assert!(gitignore_with_entry(existing).is_some(), "{existing:?}");
        }
    }
}
