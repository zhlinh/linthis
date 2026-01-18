# C#

linthis supports C# using `dotnet format` for both checking and formatting.

## Supported Extensions

- `.cs`
- `.csx`

## Tools

| Tool | Type | Description |
|------|------|-------------|
| [dotnet format](https://learn.microsoft.com/en-us/dotnet/core/tools/dotnet-format) | Checker & Formatter | .NET code formatter and analyzer |

## Installation

`dotnet format` is included with .NET SDK 6.0 and later. For earlier versions:

```bash
dotnet tool install -g dotnet-format
```

Verify installation:

```bash
dotnet format --version
```

## Configuration

### EditorConfig

Create `.editorconfig` in your project root:

```ini
# Top-most EditorConfig file
root = true

[*.cs]
# Indentation
indent_style = space
indent_size = 4

# New line preferences
end_of_line = lf
insert_final_newline = true

# C# code style settings
csharp_new_line_before_open_brace = all
csharp_new_line_before_else = true
csharp_new_line_before_catch = true
csharp_new_line_before_finally = true

# Naming conventions
dotnet_naming_rule.private_fields_should_be_camel_case.severity = warning
dotnet_naming_rule.private_fields_should_be_camel_case.symbols = private_fields
dotnet_naming_rule.private_fields_should_be_camel_case.style = camel_case_style

dotnet_naming_symbols.private_fields.applicable_kinds = field
dotnet_naming_symbols.private_fields.applicable_accessibilities = private

dotnet_naming_style.camel_case_style.capitalization = camel_case

# Code analysis
dotnet_diagnostic.CA1822.severity = warning  # Mark members as static
dotnet_diagnostic.IDE0005.severity = warning  # Remove unnecessary using
```

### .csproj Analyzers

Add analyzers to your project file:

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

## Usage

```bash
# Check C# files
linthis --lang csharp --check-only

# Format C# files
linthis --lang csharp --format-only

# Check and format
linthis --lang csharp
```

## Common Issues

### IDE0005: Remove unnecessary using

```csharp
// Bad
using System;
using System.Linq;  // unused

class Program
{
    static void Main()
    {
        Console.WriteLine("Hello");
    }
}

// Good
using System;

class Program
{
    static void Main()
    {
        Console.WriteLine("Hello");
    }
}
```

### CA1822: Mark members as static

```csharp
// Bad
class Calculator
{
    public int Add(int a, int b) => a + b;  // doesn't use instance state
}

// Good
class Calculator
{
    public static int Add(int a, int b) => a + b;
}
```

## Severity Mapping

| dotnet format Level | linthis Severity |
|--------------------|------------------|
| error | Error |
| warning | Warning |
| info | Info |

## Inline Disabling

```csharp
// Disable for file
#pragma warning disable CA1822

// Disable for block
#pragma warning disable CA1822
public int Method() => 42;
#pragma warning restore CA1822

// Disable inline with SuppressMessage attribute
[System.Diagnostics.CodeAnalysis.SuppressMessage("Performance", "CA1822")]
public int Method() => 42;
```
