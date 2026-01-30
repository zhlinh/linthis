// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT

package com.mojeter.linthis.plugin

import com.intellij.openapi.project.Project
import com.mojeter.linthis.plugin.settings.LinthisSettings
import com.redhat.devtools.lsp4ij.server.ProcessStreamConnectionProvider
import java.io.File

/**
 * Stream connection provider for the Linthis Language Server.
 *
 * This class is responsible for:
 * - Locating the linthis executable
 * - Starting the LSP server process
 * - Managing the input/output streams
 */
class LinthisStreamConnectionProvider(private val project: Project) : ProcessStreamConnectionProvider() {

    init {
        val commands = buildCommand()
        setCommands(commands)

        // Set working directory to project root
        project.basePath?.let { basePath ->
            setWorkingDirectory(basePath)
        }
    }

    private fun buildCommand(): List<String> {
        val settings = LinthisSettings.getInstance(project)
        val linthisPath = if (settings.linthisPath.isNotBlank()) {
            settings.linthisPath
        } else {
            findLinthisExecutable()
        }

        val commands = mutableListOf(linthisPath, "lsp")

        // Add --use-plugin if configured
        if (settings.usePlugin.isNotBlank()) {
            commands.add("--use-plugin")
            commands.add(settings.usePlugin)
        }

        return commands
    }

    /**
     * Find the linthis executable in common locations.
     */
    private fun findLinthisExecutable(): String {
        // Check common installation paths
        val possiblePaths = listOf(
            // User's cargo bin
            System.getProperty("user.home") + "/.cargo/bin/linthis",
            // User's local bin (pip install --user)
            System.getProperty("user.home") + "/.local/bin/linthis",
            // Homebrew on macOS
            "/opt/homebrew/bin/linthis",
            "/usr/local/bin/linthis",
            // System paths
            "/usr/bin/linthis",
            // Windows paths
            System.getenv("LOCALAPPDATA")?.let { "$it\\Programs\\Python\\Python3*\\Scripts\\linthis.exe" },
            System.getenv("USERPROFILE")?.let { "$it\\.cargo\\bin\\linthis.exe" }
        )

        for (path in possiblePaths.filterNotNull()) {
            val file = File(path)
            if (file.exists() && file.canExecute()) {
                return path
            }
        }

        // Check if linthis is in PATH
        val pathEnv = System.getenv("PATH") ?: ""
        val pathSeparator = File.pathSeparator
        val executableName = if (System.getProperty("os.name").lowercase().contains("windows")) {
            "linthis.exe"
        } else {
            "linthis"
        }

        for (dir in pathEnv.split(pathSeparator)) {
            val file = File(dir, executableName)
            if (file.exists() && file.canExecute()) {
                return file.absolutePath
            }
        }

        // Default to just "linthis" and hope it's in PATH
        return "linthis"
    }

    override fun toString(): String {
        return "Linthis Language Server: ${commands.joinToString(" ")}"
    }
}
