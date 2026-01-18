# Java 语言指南

linthis 使用 **checkstyle** 进行代码检查，使用 **google-java-format** 进行代码格式化。

## 支持的文件扩展名

- `.java`

## 必需工具

### 代码检查：checkstyle

```bash
# macOS
brew install checkstyle

# Linux
sudo apt install checkstyle

# Windows
choco install checkstyle

# 或从 https://checkstyle.org/ 下载 JAR

# 验证安装
checkstyle --version
```

### 格式化：google-java-format

```bash
# macOS
brew install google-java-format

# 下载 JAR
wget https://github.com/google/google-java-format/releases/download/v1.18.1/google-java-format-1.18.1-all-deps.jar

# 验证安装
java -jar google-java-format-1.18.1-all-deps.jar --version
```

## 配置

### 基本示例

```toml
# .linthis/config.toml

[java]
max_complexity = 20
excludes = ["*Test.java", "target/**"]
```

### 禁用特定规则

```toml
[java.rules]
disable = [
    "MagicNumber",
    "LineLength"
]
```

## 自定义规则

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

## CLI 用法

```bash
# 仅检查 Java 文件
linthis -c --lang java

# 仅格式化 Java 文件
linthis -f --lang java
```

## Checkstyle 配置

linthis 默认使用 Google 的 checkstyle 配置。要自定义，创建 `checkstyle.xml`：

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

## 常见问题

### Checkstyle 未找到

```
Warning: No java linter available for java files
  Install: brew install checkstyle
```

### 测试文件被检查

添加到排除项：

```toml
[java]
excludes = ["*Test.java", "*Tests.java", "src/test/**"]
```

## 最佳实践

1. **Google 风格**：使用 Google 的 checkstyle 配置作为基准
2. **行长度**：120 字符对现代显示器来说较为常见
3. **测试排除**：考虑对测试文件使用不同的规则
4. **IDE 集成**：配置您的 IDE 使用相同的 checkstyle 配置
