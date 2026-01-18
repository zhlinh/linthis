# C#

linthis 使用 `dotnet format` 进行 C# 代码检查和格式化。

## 支持的扩展名

- `.cs`
- `.csx`

## 工具

| 工具 | 类型 | 描述 |
|-----|-----|------|
| [dotnet format](https://learn.microsoft.com/en-us/dotnet/core/tools/dotnet-format) | 检查器和格式化器 | .NET 代码格式化和分析器 |

## 安装

`dotnet format` 包含在 .NET SDK 6.0 及更高版本中。对于早期版本：

```bash
dotnet tool install -g dotnet-format
```

验证安装：

```bash
dotnet format --version
```

## 配置

### EditorConfig

在项目根目录创建 `.editorconfig`：

```ini
# 顶级 EditorConfig 文件
root = true

[*.cs]
# 缩进
indent_style = space
indent_size = 4

# 换行偏好
end_of_line = lf
insert_final_newline = true

# C# 代码风格设置
csharp_new_line_before_open_brace = all
csharp_new_line_before_else = true
csharp_new_line_before_catch = true
csharp_new_line_before_finally = true

# 命名约定
dotnet_naming_rule.private_fields_should_be_camel_case.severity = warning
dotnet_naming_rule.private_fields_should_be_camel_case.symbols = private_fields
dotnet_naming_rule.private_fields_should_be_camel_case.style = camel_case_style

dotnet_naming_symbols.private_fields.applicable_kinds = field
dotnet_naming_symbols.private_fields.applicable_accessibilities = private

dotnet_naming_style.camel_case_style.capitalization = camel_case

# 代码分析
dotnet_diagnostic.CA1822.severity = warning  # 将成员标记为静态
dotnet_diagnostic.IDE0005.severity = warning  # 删除不必要的 using
```

### .csproj 分析器

在项目文件中添加分析器：

```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <EnforceCodeStyleInBuild>true</EnforceCodeStyleInBuild>
    <TreatWarningsAsErrors>false</TreatWarningsAsErrors>
  </PropertyGroup>

  <ItemGroup>
    <PackageReference Include="Microsoft.CodeAnalysis.NetAnalyzers" Version="8.0.0">
      <PrivateAssets>all</PrivateAssets>
      <IncludeAssets>runtime; build; native; contentfiles; analyzers</IncludeAssets>
    </PackageReference>
  </ItemGroup>
</Project>
```

## 用法

```bash
# 检查 C# 文件
linthis --lang csharp --check-only

# 格式化 C# 文件
linthis --lang csharp --format-only

# 检查并格式化
linthis --lang csharp
```

## 常见问题

### IDE0005: 删除不必要的 using

```csharp
// 错误
using System;
using System.Linq;  // 未使用

class Program
{
    static void Main()
    {
        Console.WriteLine("Hello");
    }
}

// 正确
using System;

class Program
{
    static void Main()
    {
        Console.WriteLine("Hello");
    }
}
```

### CA1822: 将成员标记为静态

```csharp
// 错误
class Calculator
{
    public int Add(int a, int b) => a + b;  // 不使用实例状态
}

// 正确
class Calculator
{
    public static int Add(int a, int b) => a + b;
}
```

## 严重性映射

| dotnet format 级别 | linthis 严重性 |
|-------------------|---------------|
| error | 错误 |
| warning | 警告 |
| info | 信息 |

## 行内禁用

```csharp
// 为文件禁用
#pragma warning disable CA1822

// 为代码块禁用
#pragma warning disable CA1822
public int Method() => 42;
#pragma warning restore CA1822

// 使用 SuppressMessage 属性行内禁用
[System.Diagnostics.CodeAnalysis.SuppressMessage("Performance", "CA1822")]
public int Method() => 42;
```
