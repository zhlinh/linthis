plugins {
    id("java")
    id("org.jetbrains.kotlin.jvm") version "1.9.25"
    id("org.jetbrains.intellij.platform") version "2.2.1"
}

group = "com.mojeter.linthis"
version = "0.1.0"

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
        pluginVerifier()
        zipSigner()
    }

    // LSP4IJ library from FalsePattern repository
    compileOnly("com.redhat.devtools.intellij:lsp4ij:0.13.0")
    // Eclipse LSP4J (LSP4IJ's dependency)
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
