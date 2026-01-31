plugins {
    id("java")
    id("org.jetbrains.kotlin.jvm") version "1.9.25"
    id("org.jetbrains.intellij.platform") version "2.2.1"
}

group = "com.mojeter.linthis"
version = "0.2.0"

repositories {
    mavenCentral()
    // FalsePattern repository for LSP4IJ
    maven {
        url = uri("https://mvn.falsepattern.com/releases/")
    }
    intellijPlatform {
        defaultRepositories()
    }
}

dependencies {
    intellijPlatform {
        intellijIdeaCommunity("2024.1")
        bundledPlugin("com.intellij.java")
        // LSP4IJ plugin from JetBrains Marketplace (required dependency)
        // The plugin() declaration makes LSP4IJ classes available for compilation
        plugin("com.redhat.devtools.lsp4ij", "0.11.0")
        pluginVerifier()
        zipSigner()
    }

    // Eclipse LSP4J (LSP4IJ's transitive dependency, needed for LSP types)
    compileOnly("org.eclipse.lsp4j:org.eclipse.lsp4j:0.23.1")
}

tasks {
    // Set the JVM compatibility versions
    withType<JavaCompile> {
        sourceCompatibility = "17"
        targetCompatibility = "17"
    }
    withType<org.jetbrains.kotlin.gradle.tasks.KotlinCompile> {
        kotlinOptions.jvmTarget = "17"
    }

    patchPluginXml {
        sinceBuild.set("241")
        untilBuild.set("251.*")
    }

    signPlugin {
        certificateChain.set(System.getenv("CERTIFICATE_CHAIN"))
        privateKey.set(System.getenv("PRIVATE_KEY"))
        password.set(System.getenv("PRIVATE_KEY_PASSWORD"))
    }
}

intellijPlatform {
    pluginConfiguration {
        name = "linthis"
        ideaVersion {
            sinceBuild = "241"
            untilBuild = "251.*"
        }
    }

    publishing {
        // Get token from: https://plugins.jetbrains.com/author/me/tokens
        token.set(System.getenv("JETBRAINS_MARKETPLACE_TOKEN"))
    }
}
