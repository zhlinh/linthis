# Scala

linthis 使用 Scalafix 进行检查，使用 Scalafmt 进行 Scala 代码格式化。

## 支持的扩展名

- `.scala`
- `.sc`

## 工具

| 工具 | 类型 | 描述 |
|-----|-----|------|
| [Scalafix](https://scalacenter.github.io/scalafix/) | 检查器 | Scala 重构和代码检查工具 |
| [Scalafmt](https://scalameta.org/scalafmt/) | 格式化器 | Scala 代码格式化工具 |

## 安装

### 使用 Coursier

```bash
# 首先安装 Coursier
curl -fL https://github.com/coursier/launchers/raw/master/cs-x86_64-apple-darwin.gz | gzip -d > cs
chmod +x cs
./cs install cs

# 安装 scalafix 和 scalafmt
cs install scalafix
cs install scalafmt
```

### 使用 sbt（项目级别）

添加到 `project/plugins.sbt`：

```scala
addSbtPlugin("ch.epfl.scala" % "sbt-scalafix" % "0.11.1")
addSbtPlugin("org.scalameta" % "sbt-scalafmt" % "2.5.2")
```

## 配置

### Scalafix

在项目根目录创建 `.scalafix.conf`：

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

在项目根目录创建 `.scalafmt.conf`：

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

## 用法

```bash
# 检查 Scala 文件
linthis --lang scala --check-only

# 格式化 Scala 文件
linthis --lang scala --format-only

# 检查并格式化
linthis --lang scala
```

## 常见问题

### 未使用的导入

```scala
// 错误
import scala.collection.mutable.ListBuffer  // 未使用

object Main extends App {
  println("Hello")
}

// 正确
object Main extends App {
  println("Hello")
}
```

### 显式结果类型

```scala
// 错误（公共方法缺少返回类型）
def calculate(x: Int) = x * 2

// 正确
def calculate(x: Int): Int = x * 2
```

## 严重性映射

| Scalafix 级别 | linthis 严重性 |
|--------------|---------------|
| error | 错误 |
| warning | 警告 |
| info | 信息 |

## 行内禁用

```scala
// scalafix:off DisableSyntax.noVars
var mutableVar = 1
// scalafix:on

// 单行
var x = 1 // scalafix:ok DisableSyntax.noVars
```
