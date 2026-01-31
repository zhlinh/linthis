// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Template generation module for linthis plugins and configurations.
//!
//! This module contains functions for generating template configuration files
//! for various programming languages and tools.

mod linter_configs;
mod plugin_templates;
mod readme;

pub use linter_configs::get_default_configs;
pub use plugin_templates::get_plugin_template_configs;
pub use readme::{generate_plugin_manifest, generate_plugin_manifest_filtered, generate_plugin_readme};
