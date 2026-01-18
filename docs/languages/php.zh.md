# PHP

linthis 使用 PHP_CodeSniffer (phpcs) 进行检查，使用 PHP-CS-Fixer 进行 PHP 代码格式化。

## 支持的扩展名

- `.php`
- `.phtml`

## 工具

| 工具 | 类型 | 描述 |
|-----|-----|------|
| [phpcs](https://github.com/squizlabs/PHP_CodeSniffer) | 检查器 | PHP 编码标准检查器 |
| [php-cs-fixer](https://cs.symfony.com/) | 格式化器 | PHP 代码格式化工具 |

## 安装

```bash
# 安装 phpcs
composer global require squizlabs/php_codesniffer

# 安装 php-cs-fixer
composer global require friendsofphp/php-cs-fixer
```

确保 Composer 的全局 bin 目录在 PATH 中：

```bash
export PATH="$PATH:$HOME/.composer/vendor/bin"
```

## 配置

### phpcs

在项目根目录创建 `phpcs.xml`：

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

在项目根目录创建 `.php-cs-fixer.php`：

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

## 用法

```bash
# 检查 PHP 文件
linthis --lang php --check-only

# 格式化 PHP 文件
linthis --lang php --format-only

# 检查并格式化
linthis --lang php
```

## 常见问题

### PSR-12 违规

```php
// 错误
class MyClass{
function myMethod(){
}}

// 正确
class MyClass
{
    public function myMethod(): void
    {
    }
}
```

### 数组语法

```php
// 错误（长语法）
$arr = array(1, 2, 3);

// 正确（短语法）
$arr = [1, 2, 3];
```

## 严重性映射

| phpcs 类型 | linthis 严重性 |
|-----------|---------------|
| ERROR | 错误 |
| WARNING | 警告 |

## 行内禁用

```php
// phpcs:disable Generic.Files.LineLength
$longLine = "This is a very long line that exceeds the limit...";
// phpcs:enable Generic.Files.LineLength

// 单行禁用
$longLine = "..."; // phpcs:ignore Generic.Files.LineLength
```
