# Scala

linthis supports Scala using Scalafix for checking and Scalafmt for formatting.

## Supported Extensions

- `.scala`
- `.sc`

## Tools

| Tool | Type | Description |
|------|------|-------------|
| [Scalafix](https://scalacenter.github.io/scalafix/) | Checker | Scala refactoring and linting tool |
| [Scalafmt](https://scalameta.org/scalafmt/) | Formatter | Scala code formatter |

## Installation

### Using Coursier

```bash
# Install Coursier first
curl -fL https://github.com/coursier/launchers/raw/master/cs-x86_64-apple-darwin.gz | gzip -d > cs
chmod +x cs
./cs install cs

# Install scalafix and scalafmt
cs install scalafix
cs install scalafmt
```

### Using sbt (project-level)

Add to `project/plugins.sbt`:

```scala
addSbtPlugin("ch.epfl.scala" % "sbt-scalafix" % "0.11.1")
addSbtPlugin("org.scalameta" % "sbt-scalafmt" % "2.5.2")
```

## Configuration

### Scalafix

Create `.scalafix.conf` in your project root:

```hocon
rules = [
  DisableSyntax,
  ExplicitResultTypes,
  NoAutoTupling,
  OrganizeImports,
  RemoveUnused
]

DisableSyntax {
  noVars = true
  noThrows = true
  noNulls = true
  noReturns = true
}

OrganizeImports {
  groups = [
    "re:javax?\\."
    "scala."
    "*"
  ]
}
```

### Scalafmt

Create `.scalafmt.conf` in your project root:

```hocon
version = "3.7.17"

runner.dialect = scala3

maxColumn = 100

indent {
  main = 2
  callSite = 2
}

align {
  preset = more
}

rewrite {
  rules = [
    RedundantBraces,
    RedundantParens,
    SortModifiers
  ]
}

newlines {
  beforeCurlyLambdaParams = multilineWithCaseOnly
}
```

## Usage

```bash
# Check Scala files
linthis --lang scala --check-only

# Format Scala files
linthis --lang scala --format-only

# Check and format
linthis --lang scala
```

## Common Issues

### Unused Imports

```scala
// Bad
import scala.collection.mutable.ListBuffer  // unused

object Main extends App {
  println("Hello")
}

// Good
object Main extends App {
  println("Hello")
}
```

### Explicit Result Types

```scala
// Bad (missing return type for public method)
def calculate(x: Int) = x * 2

// Good
def calculate(x: Int): Int = x * 2
```

## Severity Mapping

| Scalafix Level | linthis Severity |
|---------------|------------------|
| error | Error |
| warning | Warning |
| info | Info |

## Inline Disabling

```scala
// scalafix:off DisableSyntax.noVars
var mutableVar = 1
// scalafix:on

// Single line
var x = 1 // scalafix:ok DisableSyntax.noVars
```
