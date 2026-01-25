// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Prompt templates for AI-assisted fix suggestions.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Categories of lint issues for specialized prompts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueCategory {
    /// Code style issues (formatting, naming)
    Style,
    /// Security vulnerabilities
    Security,
    /// Performance problems
    Performance,
    /// Code complexity issues
    Complexity,
    /// Bug patterns and potential errors
    Bug,
    /// Deprecated API usage
    Deprecation,
    /// Type-related issues
    Type,
    /// Documentation issues
    Documentation,
    /// Best practices
    BestPractice,
    /// General/unknown category
    General,
}

impl Default for IssueCategory {
    fn default() -> Self {
        Self::General
    }
}

impl std::str::FromStr for IssueCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "style" | "formatting" | "naming" => Ok(Self::Style),
            "security" | "vulnerability" | "vuln" => Ok(Self::Security),
            "performance" | "perf" | "speed" => Ok(Self::Performance),
            "complexity" | "cyclomatic" | "cognitive" => Ok(Self::Complexity),
            "bug" | "error" | "defect" => Ok(Self::Bug),
            "deprecation" | "deprecated" => Ok(Self::Deprecation),
            "type" | "typing" => Ok(Self::Type),
            "documentation" | "doc" | "docs" => Ok(Self::Documentation),
            "best-practice" | "bestpractice" | "practice" => Ok(Self::BestPractice),
            _ => Ok(Self::General),
        }
    }
}

/// Template for generating AI prompts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    /// Template name
    pub name: String,
    /// Issue category this template is for
    pub category: IssueCategory,
    /// System prompt for the AI
    pub system_prompt: String,
    /// User prompt template with placeholders
    pub user_prompt_template: String,
    /// Additional context instructions
    pub context_instructions: Option<String>,
}

impl PromptTemplate {
    /// Create a new prompt template
    pub fn new(name: &str, category: IssueCategory, system: &str, template: &str) -> Self {
        Self {
            name: name.to_string(),
            category,
            system_prompt: system.to_string(),
            user_prompt_template: template.to_string(),
            context_instructions: None,
        }
    }

    /// Add context instructions
    pub fn with_context_instructions(mut self, instructions: &str) -> Self {
        self.context_instructions = Some(instructions.to_string());
        self
    }
}

/// Builder for constructing AI prompts
pub struct PromptBuilder {
    templates: HashMap<IssueCategory, PromptTemplate>,
    default_template: PromptTemplate,
}

impl Default for PromptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptBuilder {
    /// Create a new prompt builder with default templates
    pub fn new() -> Self {
        let mut templates = HashMap::new();

        // Style template
        templates.insert(
            IssueCategory::Style,
            PromptTemplate::new(
                "style_fix",
                IssueCategory::Style,
                SYSTEM_PROMPT_STYLE,
                USER_PROMPT_STYLE,
            ),
        );

        // Security template
        templates.insert(
            IssueCategory::Security,
            PromptTemplate::new(
                "security_fix",
                IssueCategory::Security,
                SYSTEM_PROMPT_SECURITY,
                USER_PROMPT_SECURITY,
            ),
        );

        // Performance template
        templates.insert(
            IssueCategory::Performance,
            PromptTemplate::new(
                "performance_fix",
                IssueCategory::Performance,
                SYSTEM_PROMPT_PERFORMANCE,
                USER_PROMPT_PERFORMANCE,
            ),
        );

        // Complexity template
        templates.insert(
            IssueCategory::Complexity,
            PromptTemplate::new(
                "complexity_fix",
                IssueCategory::Complexity,
                SYSTEM_PROMPT_COMPLEXITY,
                USER_PROMPT_COMPLEXITY,
            ),
        );

        // Bug template
        templates.insert(
            IssueCategory::Bug,
            PromptTemplate::new(
                "bug_fix",
                IssueCategory::Bug,
                SYSTEM_PROMPT_BUG,
                USER_PROMPT_BUG,
            ),
        );

        // Default/general template
        let default_template = PromptTemplate::new(
            "general_fix",
            IssueCategory::General,
            SYSTEM_PROMPT_GENERAL,
            USER_PROMPT_GENERAL,
        );

        Self {
            templates,
            default_template,
        }
    }

    /// Get template for a specific category
    pub fn get_template(&self, category: IssueCategory) -> &PromptTemplate {
        self.templates.get(&category).unwrap_or(&self.default_template)
    }

    /// Build a prompt for an issue
    pub fn build_prompt(
        &self,
        category: IssueCategory,
        variables: &PromptVariables,
    ) -> (String, String) {
        let template = self.get_template(category);

        let system = self.substitute_variables(&template.system_prompt, variables);
        let user = self.substitute_variables(&template.user_prompt_template, variables);

        (system, user)
    }

    /// Substitute variables in a template string
    fn substitute_variables(&self, template: &str, vars: &PromptVariables) -> String {
        let mut result = template.to_string();

        result = result.replace("{{language}}", &vars.language);
        result = result.replace("{{file_path}}", &vars.file_path);
        result = result.replace("{{line_number}}", &vars.line_number.to_string());
        result = result.replace("{{issue_message}}", &vars.issue_message);
        result = result.replace("{{rule_id}}", &vars.rule_id);
        result = result.replace("{{code_context}}", &vars.code_context);
        result = result.replace("{{issue_line}}", &vars.issue_line);

        if let Some(ref imports) = vars.imports {
            result = result.replace("{{imports}}", imports);
        } else {
            result = result.replace("{{imports}}", "");
        }

        if let Some(ref scope) = vars.scope {
            result = result.replace("{{scope}}", scope);
        } else {
            result = result.replace("{{scope}}", "");
        }

        result
    }

    /// Add or override a template
    pub fn add_template(&mut self, template: PromptTemplate) {
        self.templates.insert(template.category, template);
    }
}

/// Variables for prompt substitution
#[derive(Debug, Clone, Default)]
pub struct PromptVariables {
    /// Programming language
    pub language: String,
    /// File path
    pub file_path: String,
    /// Line number of the issue
    pub line_number: u32,
    /// Lint issue message
    pub issue_message: String,
    /// Rule ID/code
    pub rule_id: String,
    /// Code context around the issue
    pub code_context: String,
    /// The specific line with the issue
    pub issue_line: String,
    /// Import statements (if available)
    pub imports: Option<String>,
    /// Enclosing scope (function/class)
    pub scope: Option<String>,
}

impl PromptVariables {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_language(mut self, lang: &str) -> Self {
        self.language = lang.to_string();
        self
    }

    pub fn with_file_path(mut self, path: &str) -> Self {
        self.file_path = path.to_string();
        self
    }

    pub fn with_line_number(mut self, line: u32) -> Self {
        self.line_number = line;
        self
    }

    pub fn with_issue_message(mut self, msg: &str) -> Self {
        self.issue_message = msg.to_string();
        self
    }

    pub fn with_rule_id(mut self, rule: &str) -> Self {
        self.rule_id = rule.to_string();
        self
    }

    pub fn with_code_context(mut self, context: &str) -> Self {
        self.code_context = context.to_string();
        self
    }

    pub fn with_issue_line(mut self, line: &str) -> Self {
        self.issue_line = line.to_string();
        self
    }

    pub fn with_imports(mut self, imports: &str) -> Self {
        self.imports = Some(imports.to_string());
        self
    }

    pub fn with_scope(mut self, scope: &str) -> Self {
        self.scope = Some(scope.to_string());
        self
    }
}

// System prompts

const SYSTEM_PROMPT_GENERAL: &str = r#"You are an expert code reviewer and fix assistant. Your task is to analyze lint issues and suggest precise, minimal fixes.

Guidelines:
1. Provide only the fixed code, not explanations unless asked
2. Make minimal changes - fix ONLY what the error message describes
3. Preserve the original code style, formatting, and indentation
4. Consider the surrounding context when making changes
5. If the fix requires imports, include them
6. Output the fix in a code block with the appropriate language tag
7. Pay close attention to the EXACT error message - it tells you precisely what to fix

Common lint rules and their fixes:
- "Missing space after X" → Add a space AFTER the character X
- "Missing space before X" → Add a space BEFORE the character X
- "Extra space" → Remove the extra space
- "Line too long" → Break the line appropriately
- "Unused variable" → Remove or use the variable
- "Missing return type" → Add the return type annotation

Response format:
```{{language}}
// Fixed code here - preserve original indentation
```

If multiple approaches are possible, provide the most idiomatic solution for the language."#;

const SYSTEM_PROMPT_STYLE: &str = r#"You are an expert code formatter and style guide enforcer. Your task is to fix code style issues while preserving functionality.

Guidelines:
1. Follow language-specific style conventions
2. Make minimal changes to fix the style issue - focus ONLY on what the error message describes
3. Preserve existing formatting patterns where not explicitly wrong
4. Consider readability and consistency
5. Do not change logic or behavior
6. Do NOT change indentation unless the error specifically mentions indentation
7. Pay close attention to the EXACT error message - it tells you precisely what to fix

Common style rules and their fixes:
- "Missing space after X" → Add a space AFTER the character X (e.g., "Missing space after ;" means add space after semicolon)
- "Missing space before X" → Add a space BEFORE the character X
- "Extra space after X" → Remove the extra space after X
- "Line too long" → Break the line appropriately
- "Trailing whitespace" → Remove spaces/tabs at the end of line

Response format:
```{{language}}
// Fixed code here - with ONLY the specific style issue fixed
```"#;

const SYSTEM_PROMPT_SECURITY: &str = r#"You are a security expert. Your task is to fix security vulnerabilities in code while maintaining functionality.

Guidelines:
1. Apply security best practices
2. Use secure alternatives to vulnerable patterns
3. Add input validation where needed
4. Avoid introducing new vulnerabilities
5. Explain the security fix briefly if the change is non-obvious

Response format:
```{{language}}
// Fixed code here
```

Security note: Brief explanation if needed"#;

const SYSTEM_PROMPT_PERFORMANCE: &str = r#"You are a performance optimization expert. Your task is to fix performance issues while maintaining code correctness.

Guidelines:
1. Optimize only what's flagged as a performance issue
2. Prefer clarity over micro-optimizations
3. Consider memory usage and algorithmic complexity
4. Maintain code readability
5. Document any significant algorithmic changes

Response format:
```{{language}}
// Fixed code here
```"#;

const SYSTEM_PROMPT_COMPLEXITY: &str = r#"You are a code simplification expert. Your task is to reduce code complexity while maintaining functionality.

Guidelines:
1. Break down complex functions into smaller ones
2. Reduce nesting depth
3. Simplify conditional logic
4. Extract repeated code into helper functions
5. Maintain the original behavior exactly
6. Improve readability without changing semantics

Response format:
```{{language}}
// Fixed code here
```"#;

const SYSTEM_PROMPT_BUG: &str = r#"You are a debugging expert. Your task is to fix potential bugs and error patterns in code.

Guidelines:
1. Fix the specific bug pattern identified
2. Consider edge cases
3. Add null/error checks where appropriate
4. Maintain the intended behavior
5. Explain the fix briefly if the bug is subtle

Response format:
```{{language}}
// Fixed code here
```

Bug fix note: Brief explanation if needed"#;

// User prompts

const USER_PROMPT_GENERAL: &str = r#"Fix the following lint issue in {{language}} code:

File: {{file_path}}
Line: {{line_number}}
Issue: {{issue_message}}
Rule: {{rule_id}}

Code context:
```{{language}}
{{code_context}}
```

The issue is on this line (line {{line_number}}):
```{{language}}
{{issue_line}}
```

IMPORTANT: The error message "{{issue_message}}" tells you EXACTLY what to fix.
- Read the error message carefully and fix ONLY what it describes
- Do NOT change indentation or other formatting unless the error specifically mentions it
- Preserve the original code structure

Provide only the fixed line."#;

const USER_PROMPT_STYLE: &str = r#"Fix the following code style issue in {{language}}:

File: {{file_path}}
Line: {{line_number}}
Style Issue: {{issue_message}}
Rule: {{rule_id}}

Code context:
```{{language}}
{{code_context}}
```

Problem line (line {{line_number}}):
```{{language}}
{{issue_line}}
```

IMPORTANT: The error message "{{issue_message}}" tells you EXACTLY what to fix.
- If it says "Missing space after X", add a space AFTER the character X
- If it says "Missing space before X", add a space BEFORE the character X
- Do NOT change indentation or other formatting unless the error specifically mentions it
- Make the MINIMAL change to fix ONLY this specific issue

Provide ONLY the fixed line with the style issue corrected."#;

const USER_PROMPT_SECURITY: &str = r#"Fix the following security vulnerability in {{language}}:

File: {{file_path}}
Line: {{line_number}}
Security Issue: {{issue_message}}
Rule: {{rule_id}}

Vulnerable code:
```{{language}}
{{code_context}}
```

Problem line:
```{{language}}
{{issue_line}}
```

Provide secure code that fixes the vulnerability."#;

const USER_PROMPT_PERFORMANCE: &str = r#"Optimize the following performance issue in {{language}}:

File: {{file_path}}
Line: {{line_number}}
Performance Issue: {{issue_message}}
Rule: {{rule_id}}

Code to optimize:
```{{language}}
{{code_context}}
```

Problem area:
```{{language}}
{{issue_line}}
```

Provide optimized code."#;

const USER_PROMPT_COMPLEXITY: &str = r#"Simplify the following complex code in {{language}}:

File: {{file_path}}
Line: {{line_number}}
Complexity Issue: {{issue_message}}
Rule: {{rule_id}}

Complex code:
```{{language}}
{{code_context}}
```

{{#if scope}}
Full function/method:
```{{language}}
{{scope}}
```
{{/if}}

Provide simplified code that maintains the same behavior."#;

const USER_PROMPT_BUG: &str = r#"Fix the following potential bug in {{language}}:

File: {{file_path}}
Line: {{line_number}}
Bug Pattern: {{issue_message}}
Rule: {{rule_id}}

Buggy code:
```{{language}}
{{code_context}}
```

Problem line:
```{{language}}
{{issue_line}}
```

Provide corrected code that fixes the bug."#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_issue_category_parsing() {
        assert_eq!("style".parse::<IssueCategory>().unwrap(), IssueCategory::Style);
        assert_eq!("security".parse::<IssueCategory>().unwrap(), IssueCategory::Security);
        assert_eq!("performance".parse::<IssueCategory>().unwrap(), IssueCategory::Performance);
        assert_eq!("unknown".parse::<IssueCategory>().unwrap(), IssueCategory::General);
    }

    #[test]
    fn test_prompt_builder() {
        let builder = PromptBuilder::new();

        let vars = PromptVariables::new()
            .with_language("rust")
            .with_file_path("src/main.rs")
            .with_line_number(10)
            .with_issue_message("unused variable")
            .with_rule_id("W0001")
            .with_code_context("let x = 5;")
            .with_issue_line("let x = 5;");

        let (system, user) = builder.build_prompt(IssueCategory::General, &vars);

        assert!(system.contains("expert code reviewer"));
        assert!(user.contains("rust"));
        assert!(user.contains("src/main.rs"));
        assert!(user.contains("unused variable"));
    }

    #[test]
    fn test_prompt_template() {
        let template = PromptTemplate::new(
            "test",
            IssueCategory::Style,
            "System prompt",
            "User prompt",
        );

        assert_eq!(template.name, "test");
        assert_eq!(template.category, IssueCategory::Style);
    }

    #[test]
    fn test_category_specific_templates() {
        let builder = PromptBuilder::new();

        let security = builder.get_template(IssueCategory::Security);
        assert!(security.system_prompt.contains("security"));

        let performance = builder.get_template(IssueCategory::Performance);
        assert!(performance.system_prompt.contains("performance"));
    }
}
