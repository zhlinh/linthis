# Ruby

linthis supports Ruby using RuboCop for both checking and formatting.

## Supported Extensions

- `.rb`
- `.rake`
- `.gemspec`

## Tools

| Tool | Type | Description |
|------|------|-------------|
| [RuboCop](https://rubocop.org/) | Checker & Formatter | Ruby static code analyzer and formatter |

## Installation

```bash
gem install rubocop
```

Or add to your Gemfile:

```ruby
group :development do
  gem 'rubocop', require: false
end
```

Then run:

```bash
bundle install
```

## Configuration

Create `.rubocop.yml` in your project root:

```yaml
AllCops:
  TargetRubyVersion: 3.2
  NewCops: enable
  Exclude:
    - 'vendor/**/*'
    - 'db/schema.rb'

Style/StringLiterals:
  EnforcedStyle: double_quotes

Layout/LineLength:
  Max: 120

Metrics/MethodLength:
  Max: 20
```

## Usage

```bash
# Check Ruby files
linthis --lang ruby --check-only

# Format Ruby files
linthis --lang ruby --format-only

# Check and format
linthis --lang ruby
```

## Common Issues

### Layout/IndentationWidth

```ruby
# Bad (4 spaces)
def method
    body
end

# Good (2 spaces)
def method
  body
end
```

### Style/StringLiterals

```ruby
# If EnforcedStyle is double_quotes
# Bad
name = 'John'

# Good
name = "John"
```

## Severity Mapping

| RuboCop Severity | linthis Severity |
|-----------------|------------------|
| error | Error |
| fatal | Error |
| warning | Warning |
| convention | Info |
| refactor | Info |
| info | Info |

## Inline Disabling

```ruby
# rubocop:disable Style/StringLiterals
name = 'allowed'
# rubocop:enable Style/StringLiterals

# Disable for single line
name = 'allowed' # rubocop:disable Style/StringLiterals
```
