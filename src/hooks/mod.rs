// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Hook and agent-plugin source resolution.
//!
//! Implements the three-tier override system:
//! 1. Fixed-path auto-discovery (project-relative)
//! 2. TOML `[hook.*]` source entries
//! 3. Built-in generator (fallback — handled by callers)

pub mod resolver;
