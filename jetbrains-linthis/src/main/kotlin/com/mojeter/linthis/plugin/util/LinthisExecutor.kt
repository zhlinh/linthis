// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT

package com.mojeter.linthis.plugin.util

import com.intellij.openapi.diagnostic.Logger
import java.io.File
import java.util.concurrent.TimeUnit

/**
 * Utility class for executing linthis CLI commands.
 */
object LinthisExecutor {

    private val LOG = Logger.getInstance(LinthisExecutor::class.java)

    data class ExecutionResult(
        val success: Boolean,
        val output: String? = null,
        val errorMessage: String? = null,
        val exitCode: Int = 0
    )

    data class FormatResult(
        val success: Boolean,
        val formattedContent: String? = null,
        val errorMessage: String? = null
    )

    /**
     * Format a file using linthis CLI.
     *
     * @param filePath Path to the file to format
     * @param workingDir Working directory for the command
     * @param customLinthisPath Custom path to linthis executable (empty for auto-detect)
     * @param usePlugin Plugin URL to use (empty for none)
     * @return ExecutionResult with success status and any error messages
     */
    fun format(filePath: String, workingDir: String?, customLinthisPath: String?, usePlugin: String? = null): ExecutionResult {
        val linthisPath = if (customLinthisPath.isNullOrBlank()) {
            findLinthisExecutable()
        } else {
            customLinthisPath
        }

        LOG.warn("Format file: $filePath, linthisPath: $linthisPath, workingDir: $workingDir, usePlugin: $usePlugin")

        return try {
            val commands = mutableListOf(linthisPath, "-f", "-i", filePath)
            if (!usePlugin.isNullOrBlank()) {
                commands.add("--use-plugin")
                commands.add(usePlugin)
            }
            val processBuilder = ProcessBuilder(commands)

            if (workingDir != null) {
                processBuilder.directory(File(workingDir))
            }

            processBuilder.redirectErrorStream(true)

            val process = processBuilder.start()
            val output = process.inputStream.bufferedReader().readText()
            val completed = process.waitFor(30, TimeUnit.SECONDS)

            if (!completed) {
                process.destroyForcibly()
                return ExecutionResult(
                    success = false,
                    errorMessage = "Format command timed out after 30 seconds"
                )
            }

            val exitCode = process.exitValue()
            LOG.warn("Format exit code: $exitCode, output: $output")

            if (exitCode == 0) {
                ExecutionResult(success = true, output = output, exitCode = exitCode)
            } else {
                // Check for common error patterns
                val errorMessage = when {
                    output.contains("formatting errors") || output.contains("syntax error") ->
                        "Cannot format file with syntax errors. Fix syntax errors first."
                    output.isNotBlank() -> output.trim()
                    else -> "Format failed with exit code $exitCode"
                }
                LOG.warn("Format failed: $errorMessage")
                ExecutionResult(success = false, errorMessage = errorMessage, exitCode = exitCode)
            }
        } catch (e: Exception) {
            LOG.error("Format exception", e)
            ExecutionResult(
                success = false,
                errorMessage = "Failed to run linthis: ${e.message}"
            )
        }
    }

    /**
     * Format content using linthis CLI via a temp file.
     * Returns the formatted content for in-memory replacement.
     *
     * @param content The content to format
     * @param originalFilePath Original file path (used to determine file extension)
     * @param workingDir Working directory for the command
     * @param customLinthisPath Custom path to linthis executable
     * @param usePlugin Plugin URL to use (empty for none)
     * @return FormatResult with formatted content or error message
     */
    fun formatContent(
        content: String,
        originalFilePath: String,
        workingDir: String?,
        customLinthisPath: String?,
        usePlugin: String? = null
    ): FormatResult {
        val linthisPath = if (customLinthisPath.isNullOrBlank()) {
            findLinthisExecutable()
        } else {
            customLinthisPath
        }

        // Get file extension from original path
        val extension = originalFilePath.substringAfterLast('.', "")
        val tempFile = File.createTempFile("linthis_format_", if (extension.isNotEmpty()) ".$extension" else "")

        LOG.info("FormatContent: originalPath=$originalFilePath, linthisPath=$linthisPath, tempFile=${tempFile.absolutePath}, usePlugin=$usePlugin")

        return try {
            // Write content to temp file
            tempFile.writeText(content)
            LOG.info("FormatContent: wrote ${content.length} chars to temp file")

            val commands = mutableListOf(linthisPath, "-f", "-i", tempFile.absolutePath)
            if (!usePlugin.isNullOrBlank()) {
                commands.add("--use-plugin")
                commands.add(usePlugin)
            }
            val processBuilder = ProcessBuilder(commands)

            if (workingDir != null) {
                processBuilder.directory(File(workingDir))
            }

            processBuilder.redirectErrorStream(true)

            val process = processBuilder.start()
            val output = process.inputStream.bufferedReader().readText()
            val completed = process.waitFor(30, TimeUnit.SECONDS)

            if (!completed) {
                process.destroyForcibly()
                return FormatResult(
                    success = false,
                    errorMessage = "Format command timed out after 30 seconds"
                )
            }

            val exitCode = process.exitValue()

            LOG.info("FormatContent: exitCode=$exitCode, output=$output")

            if (exitCode == 0) {
                // Read formatted content from temp file
                val formattedContent = tempFile.readText()
                LOG.info("FormatContent: success, formatted ${formattedContent.length} chars")
                FormatResult(success = true, formattedContent = formattedContent)
            } else {
                val errorMessage = when {
                    output.contains("formatting errors") || output.contains("syntax error") ->
                        "Cannot format file with syntax errors. Fix syntax errors first."
                    output.isNotBlank() -> output.trim()
                    else -> "Format failed with exit code $exitCode"
                }
                LOG.warn("FormatContent: failed - $errorMessage")
                FormatResult(success = false, errorMessage = errorMessage)
            }
        } catch (e: Exception) {
            LOG.error("FormatContent: exception", e)
            FormatResult(
                success = false,
                errorMessage = "Failed to run linthis: ${e.message}"
            )
        } finally {
            // Clean up temp file
            tempFile.delete()
        }
    }

    /**
     * Find the linthis executable in common locations.
     */
    private fun findLinthisExecutable(): String {
        val possiblePaths = listOf(
            System.getProperty("user.home") + "/.cargo/bin/linthis",
            System.getProperty("user.home") + "/.local/bin/linthis",
            "/opt/homebrew/bin/linthis",
            "/usr/local/bin/linthis",
            "/usr/bin/linthis"
        )

        for (path in possiblePaths) {
            val file = File(path)
            if (file.exists() && file.canExecute()) {
                return path
            }
        }

        // Check PATH
        val pathEnv = System.getenv("PATH") ?: ""
        val executableName = if (System.getProperty("os.name").lowercase().contains("windows")) {
            "linthis.exe"
        } else {
            "linthis"
        }

        for (dir in pathEnv.split(File.pathSeparator)) {
            val file = File(dir, executableName)
            if (file.exists() && file.canExecute()) {
                return file.absolutePath
            }
        }

        // Default to just "linthis" and hope it's in PATH
        return "linthis"
    }
}
