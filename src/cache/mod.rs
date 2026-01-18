// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! File-level caching for incremental checking.
//!
//! This module provides persistent caching to skip unchanged files,
//! significantly improving performance for subsequent runs.

mod hash;
mod storage;
mod types;

pub use hash::file_hash;
pub use storage::LintCache;
pub use types::{CacheEntry, CacheStats, CachedIssue};
