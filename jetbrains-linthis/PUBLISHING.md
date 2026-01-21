# JetBrains Plugin Publishing Guide

## Prerequisites

### 1. Create JetBrains Account

1. Visit [JetBrains Account](https://account.jetbrains.com)
2. Register or login

### 2. Get Marketplace Token

1. Visit [JetBrains Marketplace Tokens](https://plugins.jetbrains.com/author/me/tokens)
2. Click **Generate Token**
3. Copy the token immediately (only shown once!)

### 3. Set Environment Variable

```bash
# Add to ~/.zshrc or ~/.bashrc
export JETBRAINS_MARKETPLACE_TOKEN="perm:xxxxxxxx"

# Reload shell config
source ~/.zshrc
```

## Pre-publish Checklist

### 1. Check plugin.xml

Ensure `src/main/resources/META-INF/plugin.xml` contains:

- ✅ `<id>` - Unique plugin ID
- ✅ `<name>` - Plugin display name
- ✅ `<vendor>` - Vendor info with email and URL
- ✅ `<description>` - Plugin description (HTML supported)
- ✅ `<change-notes>` - Version changelog
- ✅ `<depends>` - Dependencies

### 2. Check build.gradle.kts

Ensure correct configuration:

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

### 3. Add README.md

Create `README.md` with:
- Plugin introduction
- Features
- Installation instructions
- Usage guide
- Configuration options
- Screenshots

### 4. Add LICENSE

Ensure LICENSE file exists (currently MIT).

### 5. Add Icon (Optional but Recommended)

Add plugin icon at `src/main/resources/META-INF/pluginIcon.svg` (40x40 recommended).

### 6. Build and Test

```bash
# Build plugin
./gradlew buildPlugin

# Run in sandbox IDE for testing
./gradlew runIde

# Verify plugin
./gradlew verifyPlugin
```

## Publishing Steps

### Method 1: Gradle Command Line (Recommended for Updates)

#### 1. Build Plugin

```bash
./gradlew buildPlugin
```

Generated `.zip` file is in `build/distributions/`.

#### 2. Publish to Marketplace

```bash
# Set token and publish
export JETBRAINS_MARKETPLACE_TOKEN="perm:xxxxxxxx"
./gradlew publishPlugin

# Or in one line
JETBRAINS_MARKETPLACE_TOKEN="perm:xxxxxxxx" ./gradlew publishPlugin
```

### Method 2: Web Upload (Required for First Publish)

1. Visit [JetBrains Marketplace](https://plugins.jetbrains.com)
2. Login with JetBrains account
3. Click avatar → **Upload plugin**
4. Upload `build/distributions/linthis-0.0.1.zip`
5. Fill in plugin information
6. Add screenshots (recommended)
7. Submit for review

**Note**: First-time submissions require JetBrains team review (usually 1-2 business days). After approval, updates can be published automatically via Gradle.

### 3. Verify Publication

After publishing, visit:
- `https://plugins.jetbrains.com/plugin/<plugin-id>`

## Updating Published Plugin

### 1. Update Version

Edit `build.gradle.kts`:

```kotlin
version = "0.0.2"
```

### 2. Update Change Notes

Edit `plugin.xml`:

```xml
<change-notes><![CDATA[
<h3>0.0.2</h3>
<ul>
    <li>New feature X</li>
    <li>Bug fix Y</li>
</ul>
]]></change-notes>
```

### 3. Build and Publish

```bash
./gradlew buildPlugin
./gradlew publishPlugin
```

## Common Issues

### 1. Build Failed: Missing Dependencies

**Solution**: Ensure LSP4IJ dependency is correctly configured:

```kotlin
dependencies {
    compileOnly("com.redhat.devtools.intellij:lsp4ij:0.13.0")
}
```

### 2. Publish Failed: Token Invalid

**Solution**:
1. Visit [Tokens Page](https://plugins.jetbrains.com/author/me/tokens)
2. Generate new token
3. Update environment variable

### 3. Publish Failed: Plugin Not Approved

**Solution**: First-time publish must be done via web upload and wait for approval.

### 4. Build Failed: IDE Running

**Solution**: Close all IntelliJ IDEA instances, then run:

```bash
./gradlew buildPlugin --no-daemon
```

### 5. Compatibility Issues

**Solution**: Adjust `sinceBuild` and `untilBuild` in `build.gradle.kts`:

```kotlin
ideaVersion {
    sinceBuild = "241"      // IntelliJ 2024.1+
    untilBuild = "251.*"    // Up to 2025.1.x
}
```

## Post-publish Management

### View Statistics

Visit [Marketplace Publisher](https://plugins.jetbrains.com/author/me) to view:
- Downloads
- Ratings
- Reviews
- Installation trends

### Respond to Feedback

- Monitor GitHub Issues
- Reply to Marketplace reviews
- Fix bugs promptly

### Version Management

Follow [Semantic Versioning](https://semver.org/):
- **Patch** (0.0.x): Bug fixes
- **Minor** (0.x.0): New features, backward compatible
- **Major** (x.0.0): Breaking changes

## Complete Publish Script

Create `scripts/publish.sh`:

```bash
#!/bin/bash
set -e

# Check token
if [ -z "$JETBRAINS_MARKETPLACE_TOKEN" ]; then
    echo "Error: JETBRAINS_MARKETPLACE_TOKEN not set"
    exit 1
fi

# Check for uncommitted changes
if [[ -n $(git status -s) ]]; then
    echo "Error: Working directory not clean"
    exit 1
fi

# Build
echo "Building..."
./gradlew clean buildPlugin

# Verify
echo "Verifying..."
./gradlew verifyPlugin

# Publish
echo "Publishing..."
./gradlew publishPlugin

echo "Done!"
```

Usage:
```bash
chmod +x scripts/publish.sh
./scripts/publish.sh
```

## Quick Commands

```bash
# Build only
./gradlew buildPlugin

# Test in sandbox
./gradlew runIde

# Verify plugin compatibility
./gradlew verifyPlugin

# Publish (requires token)
./gradlew publishPlugin

# Clean and rebuild
./gradlew clean buildPlugin
```

## Reference Links

- [JetBrains Plugin Development](https://plugins.jetbrains.com/docs/intellij/welcome.html)
- [IntelliJ Platform Gradle Plugin](https://plugins.jetbrains.com/docs/intellij/tools-intellij-platform-gradle-plugin.html)
- [Plugin Configuration File](https://plugins.jetbrains.com/docs/intellij/plugin-configuration-file.html)
- [Marketplace Publishing](https://plugins.jetbrains.com/docs/marketplace/plugin-upload.html)
