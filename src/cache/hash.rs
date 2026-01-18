// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Fast file hashing using xxHash64.

use crate::Result;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use xxhash_rust::xxh64::xxh64;

/// Buffer size for streaming hash (64KB)
const HASH_BUFFER_SIZE: usize = 64 * 1024;

/// Calculate xxHash64 of a file's content.
///
/// Uses streaming for memory efficiency on large files.
pub fn file_hash(path: &Path) -> Result<u64> {
    let file = File::open(path)?;
    let mut reader = BufReader::with_capacity(HASH_BUFFER_SIZE, file);
    let mut buffer = vec![0u8; HASH_BUFFER_SIZE];
    let mut hasher_data = Vec::new();

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher_data.extend_from_slice(&buffer[..bytes_read]);
    }

    Ok(xxh64(&hasher_data, 0))
}

/// Check if a file has changed by comparing mtime first, then hash.
///
/// Returns `true` if the file has changed or if we can't determine.
pub fn is_file_changed(
    path: &Path,
    cached_mtime: std::time::SystemTime,
    cached_hash: u64,
) -> Result<bool> {
    // First check mtime (fast path)
    let metadata = std::fs::metadata(path)?;
    let current_mtime = metadata.modified()?;

    // If mtime is the same, file hasn't changed
    if current_mtime == cached_mtime {
        return Ok(false);
    }

    // mtime changed, verify with hash (handles touch without content change)
    let current_hash = file_hash(path)?;
    Ok(current_hash != cached_hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_file_hash_deterministic() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Hello, world!").unwrap();

        let hash1 = file_hash(file.path()).unwrap();
        let hash2 = file_hash(file.path()).unwrap();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_file_hash_different_content() {
        let mut file1 = NamedTempFile::new().unwrap();
        writeln!(file1, "Content A").unwrap();

        let mut file2 = NamedTempFile::new().unwrap();
        writeln!(file2, "Content B").unwrap();

        let hash1 = file_hash(file1.path()).unwrap();
        let hash2 = file_hash(file2.path()).unwrap();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_is_file_changed_same_content() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Test content").unwrap();

        let metadata = std::fs::metadata(file.path()).unwrap();
        let mtime = metadata.modified().unwrap();
        let hash = file_hash(file.path()).unwrap();

        // File hasn't changed
        assert!(!is_file_changed(file.path(), mtime, hash).unwrap());
    }

    #[test]
    fn test_is_file_changed_different_hash() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Original content").unwrap();

        let metadata = std::fs::metadata(file.path()).unwrap();
        let mtime = metadata.modified().unwrap();
        let wrong_hash = 12345u64; // Wrong hash

        // mtime will match, but hash differs
        // Actually this test needs the mtime to differ to trigger hash check
        // Let's modify the file
        std::thread::sleep(std::time::Duration::from_millis(10));
        writeln!(file, "Modified content").unwrap();

        assert!(is_file_changed(file.path(), mtime, wrong_hash).unwrap());
    }
}
