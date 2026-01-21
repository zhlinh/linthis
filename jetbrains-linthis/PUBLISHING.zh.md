# JetBrains 插件发布指南

## 发布前准备

### 1. 创建 JetBrains 账号

1. 访问 [JetBrains Account](https://account.jetbrains.com)
2. 注册或登录

### 2. 获取 Marketplace Token

1. 访问 [JetBrains Marketplace Tokens](https://plugins.jetbrains.com/author/me/tokens)
2. 点击 **Generate Token**（生成令牌）
3. 立即复制令牌（只显示一次！）

### 3. 设置环境变量

```bash
# 添加到 ~/.zshrc 或 ~/.bashrc
export JETBRAINS_MARKETPLACE_TOKEN="perm:xxxxxxxx"

# 重新加载配置
source ~/.zshrc
```

## 发布前检查清单

### 1. 检查 plugin.xml

确保 `src/main/resources/META-INF/plugin.xml` 包含：

- ✅ `<id>` - 唯一插件 ID
- ✅ `<name>` - 插件显示名称
- ✅ `<vendor>` - 厂商信息（包含邮箱和 URL）
- ✅ `<description>` - 插件描述（支持 HTML）
- ✅ `<change-notes>` - 版本更新日志
- ✅ `<depends>` - 依赖项

### 2. 检查 build.gradle.kts

确保配置正确：

```kotlin
group = "com.mojeter.linthis"
version = "0.0.1"

intellijPlatform {
    pluginConfiguration {
        name = "linthis"
        ideaVersion {
            sinceBuild = "241"
            untilBuild = "251.*"
        }
    }

    publishing {
        token.set(System.getenv("JETBRAINS_MARKETPLACE_TOKEN"))
    }
}
```

### 3. 添加 README.md

创建 `README.md`，包含：
- 插件介绍
- 功能特性
- 安装方法
- 使用说明
- 配置选项
- 截图

### 4. 添加 LICENSE

确保有 LICENSE 文件（当前是 MIT）。

### 5. 添加图标（可选但推荐）

在 `src/main/resources/META-INF/pluginIcon.svg` 添加插件图标（建议 40x40）。

### 6. 构建和测试

```bash
# 构建插件
./gradlew buildPlugin

# 在沙盒 IDE 中测试
./gradlew runIde

# 验证插件
./gradlew verifyPlugin
```

## 发布步骤

### 方法一：Gradle 命令行发布（推荐用于更新）

#### 1. 构建插件

```bash
./gradlew buildPlugin
```

生成的 `.zip` 文件在 `build/distributions/` 目录下。

#### 2. 发布到 Marketplace

```bash
# 设置令牌并发布
export JETBRAINS_MARKETPLACE_TOKEN="perm:xxxxxxxx"
./gradlew publishPlugin

# 或者一行命令
JETBRAINS_MARKETPLACE_TOKEN="perm:xxxxxxxx" ./gradlew publishPlugin
```

### 方法二：网页上传（首次发布必须）

1. 访问 [JetBrains Marketplace](https://plugins.jetbrains.com)
2. 使用 JetBrains 账号登录
3. 点击头像 → **Upload plugin**（上传插件）
4. 上传 `build/distributions/linthis-0.0.1.zip`
5. 填写插件信息
6. 添加截图（推荐）
7. 提交审核

**注意**：首次提交需要 JetBrains 团队审核（通常 1-2 个工作日）。审核通过后，后续更新可以通过 Gradle 自动发布。

### 3. 验证发布

发布后访问：
- `https://plugins.jetbrains.com/plugin/<插件ID>`

## 更新已发布的插件

### 1. 更新版本号

编辑 `build.gradle.kts`：

```kotlin
version = "0.0.2"
```

### 2. 更新更新日志

编辑 `plugin.xml`：

```xml
<change-notes><![CDATA[
<h3>0.0.2</h3>
<ul>
    <li>新功能 X</li>
    <li>修复 Bug Y</li>
</ul>
]]></change-notes>
```

### 3. 构建并发布

```bash
./gradlew buildPlugin
./gradlew publishPlugin
```

## 常见问题

### 1. 构建失败：缺少依赖

**解决方法**：确保 LSP4IJ 依赖配置正确：

```kotlin
dependencies {
    compileOnly("com.redhat.devtools.intellij:lsp4ij:0.13.0")
}
```

### 2. 发布失败：Token 无效

**解决方法**：
1. 访问 [Tokens 页面](https://plugins.jetbrains.com/author/me/tokens)
2. 生成新的令牌
3. 更新环境变量

### 3. 发布失败：插件未审核通过

**解决方法**：首次发布必须通过网页上传并等待审核。

### 4. 构建失败：IDE 正在运行

**解决方法**：关闭所有 IntelliJ IDEA 实例，然后运行：

```bash
./gradlew buildPlugin --no-daemon
```

### 5. 兼容性问题

**解决方法**：调整 `build.gradle.kts` 中的 `sinceBuild` 和 `untilBuild`：

```kotlin
ideaVersion {
    sinceBuild = "241"      // IntelliJ 2024.1+
    untilBuild = "251.*"    // 最高到 2025.1.x
}
```

## 发布后管理

### 查看统计

访问 [Marketplace Publisher](https://plugins.jetbrains.com/author/me) 查看：
- 下载量
- 评分
- 评论
- 安装趋势

### 回应用户反馈

- 监控 GitHub Issues
- 回复 Marketplace 评论
- 及时修复 Bug

### 版本管理

遵循 [语义化版本](https://semver.org/lang/zh-CN/)：
- **Patch 补丁版本** (0.0.x): Bug 修复
- **Minor 次版本** (0.x.0): 新功能，向后兼容
- **Major 主版本** (x.0.0): 破坏性变更

## 完整发布脚本

创建 `scripts/publish.sh`：

```bash
#!/bin/bash
set -e

# 检查令牌
if [ -z "$JETBRAINS_MARKETPLACE_TOKEN" ]; then
    echo "错误：未设置 JETBRAINS_MARKETPLACE_TOKEN"
    exit 1
fi

# 检查是否有未提交的更改
if [[ -n $(git status -s) ]]; then
    echo "错误：工作目录不干净，有未提交的更改"
    exit 1
fi

# 构建
echo "正在构建..."
./gradlew clean buildPlugin

# 验证
echo "正在验证..."
./gradlew verifyPlugin

# 发布
echo "正在发布..."
./gradlew publishPlugin

echo "发布完成！"
```

使用：
```bash
chmod +x scripts/publish.sh
./scripts/publish.sh
```

## 快速命令

```bash
# 仅构建
./gradlew buildPlugin

# 在沙盒中测试
./gradlew runIde

# 验证插件兼容性
./gradlew verifyPlugin

# 发布（需要令牌）
./gradlew publishPlugin

# 清理并重新构建
./gradlew clean buildPlugin
```

## 参考链接

- [JetBrains 插件开发文档](https://plugins.jetbrains.com/docs/intellij/welcome.html)
- [IntelliJ Platform Gradle 插件](https://plugins.jetbrains.com/docs/intellij/tools-intellij-platform-gradle-plugin.html)
- [插件配置文件](https://plugins.jetbrains.com/docs/intellij/plugin-configuration-file.html)
- [Marketplace 发布指南](https://plugins.jetbrains.com/docs/marketplace/plugin-upload.html)
