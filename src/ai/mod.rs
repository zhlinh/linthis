// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! AI-assisted fix suggestions module.
//!
//! This module provides AI-powered code fix suggestions for lint issues.
//! It supports multiple AI providers and can generate intelligent fixes
//! based on the context of the code.
//!
//! # Example
//!
//! ```rust,ignore
//! use linthis::ai::{AiSuggester, SuggestionOptions, AiProvider};
//! use linthis::utils::types::LintIssue;
//!
//! // Create suggester with a provider
//! let suggester = AiSuggester::with_provider(AiProvider::default());
//! let options = SuggestionOptions::default();
//!
//! // Get fix suggestion for an issue
//! let suggestion = suggester.suggest_fix(&issue, &source_code, &options);
//! ```

mod provider;
mod suggestions;
mod context;
mod prompts;

pub use provider::{AiProvider, AiProviderConfig, AiProviderKind, AiProviderTrait};
pub use suggestions::{
    AiSuggester, FixSuggestion, SuggestionOptions, SuggestionResult, SuggestionsReport,
};
pub use context::{CodeContext, ContextOptions, extract_context};
pub use prompts::{PromptTemplate, PromptBuilder};
