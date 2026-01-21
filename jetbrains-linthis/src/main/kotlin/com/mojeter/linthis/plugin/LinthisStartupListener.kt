// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT

package com.mojeter.linthis.plugin

import com.intellij.ide.AppLifecycleListener
import com.intellij.notification.NotificationGroupManager
import com.intellij.notification.NotificationType
import com.intellij.openapi.application.ApplicationManager
import java.io.File

/**
 * Startup listener for the Linthis plugin.
 *
 * Checks if the linthis CLI is available and notifies the user
 * with helpful installation instructions if not found.
 */
class LinthisStartupListener : AppLifecycleListener {

    override fun appFrameCreated(commandLineArgs: MutableList<String>) {
        ApplicationManager.getApplication().executeOnPooledThread {
            // Check if linthis CLI is installed
            if (!isLinthisInstalled()) {
                showLinthisCliNotification()
            }
        }
    }

    private fun isLinthisInstalled(): Boolean {
        // Check common installation paths
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
                return true
            }
        }

        // Check PATH
        val pathEnv = System.getenv("PATH") ?: return false
        val executableName = if (System.getProperty("os.name").lowercase().contains("windows")) {
            "linthis.exe"
        } else {
            "linthis"
        }

        for (dir in pathEnv.split(File.pathSeparator)) {
            val file = File(dir, executableName)
            if (file.exists() && file.canExecute()) {
                return true
            }
        }

        return false
    }

    private fun showLinthisCliNotification() {
        ApplicationManager.getApplication().invokeLater {
            NotificationGroupManager.getInstance()
                .getNotificationGroup("Linthis Notifications")
                .createNotification(
                    "linthis CLI not found",
                    """
                    The <b>linthis</b> command-line tool was not found. Please install it:
                    <br><br>
                    <code>pip install linthis</code>
                    <br>or<br>
                    <code>cargo install linthis</code>
                    <br><br>
                    After installation, restart the IDE.
                    """.trimIndent(),
                    NotificationType.WARNING
                )
                .notify(null)
        }
    }
}
