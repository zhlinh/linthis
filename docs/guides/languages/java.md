# Java Language Guide

linthis uses **checkstyle** for linting and **google-java-format** for formatting Java code.

## Supported File Extensions

- `.java`

## Required Tools

### Linter: checkstyle

```bash
# macOS
brew install checkstyle

# Linux
sudo apt install checkstyle

# Windows
choco install checkstyle

# Or download JAR from https://checkstyle.org/

# Verify installation
checkstyle --version
```

### Formatter: google-java-format

```bash
# macOS
brew install google-java-format

# Download JAR
wget https://github.com/google/google-java-format/releases/download/v1.18.1/google-java-format-1.18.1-all-deps.jar

# Verify installation
java -jar google-java-format-1.18.1-all-deps.jar --version
```

## Configuration

### Basic Example

```toml
# .linthis/config.toml

[java]
max_complexity = 20
excludes = ["*Test.java", "target/**"]
```

### Disable Specific Rules

```toml
[java.rules]
disable = [
    "MagicNumber",
    "LineLength"
]
```

## Custom Rules

```toml
[[rules.custom]]
code = "java/no-system-out"
pattern = "System\\.(out|err)\\.print"
message = "Use logging framework instead of System.out"
severity = "warning"
suggestion = "Use SLF4J or Log4j"
languages = ["java"]

[[rules.custom]]
code = "java/no-wildcard-import"
pattern = "import\\s+[\\w.]+\\.\\*;"
message = "Avoid wildcard imports"
severity = "info"
languages = ["java"]
```

## CLI Usage

```bash
# Check Java files only
linthis -c --lang java

# Format Java files only
linthis -f --lang java
```

## Checkstyle Configuration

linthis uses Google's checkstyle configuration by default. To customize, create `checkstyle.xml`:

```xml
<?xml version="1.0"?>
<!DOCTYPE module PUBLIC
    "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
    "https://checkstyle.org/dtds/configuration_1_3.dtd">
<module name="Checker">
    <module name="TreeWalker">
        <module name="LineLength">
            <property name="max" value="120"/>
        </module>
    </module>
</module>
```

## Common Issues

### Checkstyle not found

```
Warning: No java linter available for java files
  Install: brew install checkstyle
```

### Test files being checked

Add to excludes:

```toml
[java]
excludes = ["*Test.java", "*Tests.java", "src/test/**"]
```

## Best Practices

1. **Google style**: Use Google's checkstyle configuration as a baseline
2. **Line length**: 120 characters is common for modern displays
3. **Test exclusions**: Consider different rules for test files
4. **IDE integration**: Configure your IDE to use the same checkstyle config
