# PHP

linthis supports PHP using PHP_CodeSniffer (phpcs) for checking and PHP-CS-Fixer for formatting.

## Supported Extensions

- `.php`
- `.phtml`

## Tools

| Tool | Type | Description |
|------|------|-------------|
| [phpcs](https://github.com/squizlabs/PHP_CodeSniffer) | Checker | PHP coding standards checker |
| [php-cs-fixer](https://cs.symfony.com/) | Formatter | PHP code formatter |

## Installation

```bash
# Install phpcs
composer global require squizlabs/php_codesniffer

# Install php-cs-fixer
composer global require friendsofphp/php-cs-fixer
```

Make sure Composer's global bin directory is in your PATH:

```bash
export PATH="$PATH:$HOME/.composer/vendor/bin"
```

## Configuration

### phpcs

Create `phpcs.xml` in your project root:

```xml
<?xml version="1.0"?>
<ruleset name="MyProject">
    <description>My project coding standards</description>

    <file>src</file>
    <file>tests</file>

    <exclude-pattern>vendor/*</exclude-pattern>

    <rule ref="PSR12"/>

    <rule ref="Generic.Files.LineLength">
        <properties>
            <property name="lineLimit" value="120"/>
        </properties>
    </rule>
</ruleset>
```

### php-cs-fixer

Create `.php-cs-fixer.php` in your project root:

```php
<?php

$finder = PhpCsFixer\Finder::create()
    ->in(__DIR__ . '/src')
    ->in(__DIR__ . '/tests');

return (new PhpCsFixer\Config())
    ->setRules([
        '@PSR12' => true,
        'array_syntax' => ['syntax' => 'short'],
        'ordered_imports' => true,
    ])
    ->setFinder($finder);
```

## Usage

```bash
# Check PHP files
linthis --lang php --check-only

# Format PHP files
linthis --lang php --format-only

# Check and format
linthis --lang php
```

## Common Issues

### PSR-12 Violations

```php
// Bad
class MyClass{
function myMethod(){
}}

// Good
class MyClass
{
    public function myMethod(): void
    {
    }
}
```

### Array Syntax

```php
// Bad (long syntax)
$arr = array(1, 2, 3);

// Good (short syntax)
$arr = [1, 2, 3];
```

## Severity Mapping

| phpcs Type | linthis Severity |
|-----------|------------------|
| ERROR | Error |
| WARNING | Warning |

## Inline Disabling

```php
// phpcs:disable Generic.Files.LineLength
$longLine = "This is a very long line that exceeds the limit...";
// phpcs:enable Generic.Files.LineLength

// Single line disable
$longLine = "..."; // phpcs:ignore Generic.Files.LineLength
```
