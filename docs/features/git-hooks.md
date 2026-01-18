# Global Git Hook Template

## Overview

linthis supports creating global Git hook templates, allowing all newly created Git repositories to automatically include linthis pre-commit hooks - "configure once, benefit forever".

## Quick Start

### 1. Create Global Hook Template

```bash
# Create global config + Git hook template
linthis init -g --hook-type git

# Or shorthand (-g defaults to git hook template)
linthis init -g
```

Output:
```
✓ Created /Users/username/.linthis/config.toml
✓ Created /Users/username/.linthis/.git-template/hooks/pre-commit
✓ Configured git global template: init.templateDir
  All new repositories will include this hook

Next steps:
  • New repositories will automatically include the linthis hook
  • For existing repositories, run: git init
  • Or manually copy the hook to .git/hooks/pre-commit
```

### 2. Automatic Application

When creating new repositories, hooks are automatically included:

```bash
# Create new repository
mkdir my-project
cd my-project
git init

# Hook is already created
ls .git/hooks/pre-commit  # ✓ exists
```

### 3. Apply to Existing Repositories

For existing repositories, run `git init` to reapply the template:

```bash
cd existing-project
git init  # Copies template hooks
```

## Directory Structure

Global hook template is stored at:
```
~/.linthis/
├── config.toml              # Global config
└── .git-template/           # Git template directory
    └── hooks/
        └── pre-commit       # Pre-commit hook template
```

## Git Configuration

linthis automatically configures Git global settings:

```bash
# View configuration
git config --global --get init.templateDir
# Output: /Users/username/.linthis/.git-template
```

This setting makes `git init` and `git clone` automatically apply the template.

## Default Hook Content

```bash
#!/bin/sh
# linthis pre-commit hook (global template)
linthis -s -c -f -w
```

Parameters:
- `-s`: Check staged files only
- `-c`: Run checks
- `-f`: Run formatting
- `-w`: Treat warnings as errors (strict mode)

## Advanced Usage

### Custom Hook Behavior

#### Check-Only Mode

```bash
linthis init -g --hook-type git --hook-check-only
```

Generated hook:
```bash
#!/bin/sh
# linthis pre-commit hook (global template)
linthis -s -c -w
```

#### Format-Only Mode

```bash
linthis init -g --hook-type git --hook-format-only
```

Generated hook:
```bash
#!/bin/sh
# linthis pre-commit hook (global template)
linthis -s -f -w
```

### Force Overwrite

If template already exists, use `--force` to overwrite:

```bash
linthis init -g --hook-type git --force
```

### Disable Hook Creation

Create only global config without hook template:

```bash
linthis init -g --no-hook
```

## Global vs Project-Level Hooks

| Feature | Global Template (`-g`) | Project-Level Hook |
|---------|------------------------|-------------------|
| Scope | All new repositories | Current project only |
| Location | `~/.linthis/.git-template/` | `.git/hooks/` |
| Committable | No | No (.git not tracked) |
| Team sharing | No | Requires prek/pre-commit |
| Use case | Personal dev environment | Single project |

## Team Collaboration Recommendations

### Individual Developers

Use global template:
```bash
linthis init -g
```

### Team Projects

Use prek or pre-commit (config is committable):

```bash
# In project directory
linthis init --hook-type prek
# Or
linthis init --hook-type pre-commit
```

This way, config files can be committed to the repository for team sharing.

## FAQ

### Q1: How to uninstall global hook template?

```bash
# Delete template directory
rm -rf ~/.linthis/.git-template

# Unset git config
git config --global --unset init.templateDir
```

### Q2: How to skip linthis for certain repositories?

**Method 1** (Recommended): Don't create linthis config file

The hook auto-detects; if no linthis config exists, linthis won't run.

**Method 2**: Delete the hook

```bash
# Delete hook in project
cd my-project
rm .git/hooks/pre-commit
```

### Q3: Can I use both global template and project-level prek?

Yes, but not recommended. Suggested approach:
- Personal projects: Use global template
- Team projects: Use project-level prek/pre-commit

### Q4: Hook not executing?

Check permissions:
```bash
ls -l ~/.linthis/.git-template/hooks/pre-commit
# Should show -rwxr-xr-x (executable)

# If not executable, set manually
chmod +x ~/.linthis/.git-template/hooks/pre-commit
```

### Q5: Why does `-g --hook-type prek` show a warning?

Global template only supports git hook type because:
- prek/pre-commit requires running `prek install` in project directory
- Their config files (.pre-commit-config.yaml) are project-level

For prek/pre-commit, use in project directory:
```bash
linthis init --hook-type prek
```

### Q6: How to coexist with other hook tools (husky, pre-commit)?

**Option 1**: Use `.git/hooks/pre-commit.local`

Global hook automatically chains to `.local` file:

```bash
# Put other tool commands in .local file
cat > .git/hooks/pre-commit.local << 'EOF'
#!/bin/sh
# Run other checks
npm run lint
pytest
EOF
chmod +x .git/hooks/pre-commit.local
```

Execution order:
1. linthis (if config exists)
2. Commands in .local

**Option 2**: Disable global hook, use tool's own hook

```bash
# Don't create linthis config in project
# Global hook will skip, won't affect other tools
```

### Q7: Will it affect projects not using linthis?

**No!** Hook uses smart detection:

- Only runs if linthis config file exists
- Projects without config are completely unaffected
- Verified: Creating new project without linthis config, hook doesn't execute any linthis commands

## Smart Execution Mechanism

The global hook template uses **smart conditional execution**, never interfering with other projects:

### Workflow

1. **Create template**: linthis creates smart pre-commit in `~/.linthis/.git-template/hooks/`
2. **Configure Git**: Sets `git config --global init.templateDir`
3. **Auto-apply**: `git init` copies template directory content to `.git/`
4. **Hook execution**: Git automatically runs `.git/hooks/pre-commit` on commit

### Smart Detection Logic

Hook executes in this order:

```bash
1. Check if project has linthis config:
   - .linthis/config.toml
   - .linthis.toml
   - linthis.toml

2. If config exists → Run linthis
   If no config → Skip linthis (no impact)

3. Check for project-specific hook:
   - .git/hooks/pre-commit.local

4. If exists → Chain execute
```

### Hook Source Code

Generated smart hook content:

```bash
#!/bin/sh
# linthis pre-commit hook (global template)
# This hook is installed globally and will only run if the project uses linthis

# Check if this project uses linthis
if [ -f ".linthis/config.toml" ] || [ -f ".linthis.toml" ] || [ -f "linthis.toml" ]; then
    # Run linthis for this project
    linthis -s -c -f -w || exit 1
fi

# Chain to project-specific hook if it exists
# This allows projects to have their own hooks alongside linthis
if [ -f ".git/hooks/pre-commit.local" ]; then
    .git/hooks/pre-commit.local || exit 1
fi
```

## Usage Scenarios

### Scenario 1: Project Using linthis

```bash
my-rust-project/
├── .linthis/
│   └── config.toml     # ✓ Has config
└── .git/
    └── hooks/
        └── pre-commit  # Will run linthis
```

**Result**: Automatically runs linthis check and format on commit

### Scenario 2: Project Not Using linthis

```bash
other-project/
└── .git/
    └── hooks/
        └── pre-commit  # ✗ No linthis config
```

**Result**: Hook skips linthis, no impact on project

### Scenario 3: Project with Additional Hook Needs

```bash
complex-project/
├── .linthis/
│   └── config.toml       # ✓ Has config
└── .git/
    └── hooks/
        ├── pre-commit        # Runs linthis
        └── pre-commit.local  # Then runs this
```

**Result**: Runs linthis first, then project-specific checks

### Scenario 4: Project Using Other Hook Tools

If project uses husky, pre-commit, etc.:

```bash
# Option 1: Remove global hook, use tool's own hook
rm .git/hooks/pre-commit
# Then husky/pre-commit will create its own hook

# Option 2: Put tool commands in pre-commit.local
mv .git/hooks/pre-commit .git/hooks/pre-commit.backup
# Create pre-commit.local to call other tools
```

## References

- [Git documentation - init.templateDir](https://git-scm.com/docs/git-init#_template_directory)
