# Ruby

linthis 使用 RuboCop 进行 Ruby 代码检查和格式化。

## 支持的扩展名

- `.rb`
- `.rake`
- `.gemspec`

## 工具

| 工具 | 类型 | 描述 |
|-----|-----|------|
| [RuboCop](https://rubocop.org/) | 检查器和格式化器 | Ruby 静态代码分析器和格式化工具 |

## 安装

```bash
gem install rubocop
```

或添加到 Gemfile：

```ruby
group :development do
  gem 'rubocop', require: false
end
```

然后运行：

```bash
bundle install
```

## 配置

在项目根目录创建 `.rubocop.yml`：

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

## 用法

```bash
# 检查 Ruby 文件
linthis --lang ruby --check-only

# 格式化 Ruby 文件
linthis --lang ruby --format-only

# 检查并格式化
linthis --lang ruby
```

## 常见问题

### Layout/IndentationWidth

```ruby
# 错误（4 空格）
def method
    body
end

# 正确（2 空格）
def method
  body
end
```

### Style/StringLiterals

```ruby
# 如果 EnforcedStyle 是 double_quotes
# 错误
name = 'John'

# 正确
name = "John"
```

## 严重性映射

| RuboCop 严重性 | linthis 严重性 |
|---------------|---------------|
| error | 错误 |
| fatal | 错误 |
| warning | 警告 |
| convention | 信息 |
| refactor | 信息 |
| info | 信息 |

## 行内禁用

```ruby
# rubocop:disable Style/StringLiterals
name = 'allowed'
# rubocop:enable Style/StringLiterals

# 单行禁用
name = 'allowed' # rubocop:disable Style/StringLiterals
```
