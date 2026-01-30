// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT

package com.mojeter.linthis.plugin.actions

import com.intellij.notification.NotificationGroupManager
import com.intellij.notification.NotificationType
import com.intellij.openapi.actionSystem.ActionUpdateThread
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.actionSystem.CommonDataKeys
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.diagnostic.Logger
import com.intellij.openapi.fileEditor.FileDocumentManager
import com.intellij.openapi.project.DumbAware
import com.intellij.openapi.vfs.VirtualFile
import com.mojeter.linthis.plugin.settings.LinthisSettings
import com.mojeter.linthis.plugin.util.LinthisExecutor

/**
 * Action to manually trigger formatting on the current file.
 *
 * This action uses the linthis CLI with `-f -i <file_path>` to format the file in-place.
 */
class LinthisFormatAction : AnAction(), DumbAware {

    companion object {
        private val LOG = Logger.getInstance(LinthisFormatAction::class.java)
    }

    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project ?: return
        val editor = e.getData(CommonDataKeys.EDITOR) ?: return
        val file = e.getData(CommonDataKeys.VIRTUAL_FILE) ?: return

        LOG.warn("Format action triggered for: ${file.path}")

        // Save the file first to ensure linthis formats the latest content
        FileDocumentManager.getInstance().saveDocument(editor.document)

        val settings = LinthisSettings.getInstance(project)
        LOG.warn("Settings - linthisPath: '${settings.linthisPath}', formatOnSave: ${settings.formatOnSave}")

        val document = editor.document

        // Run linthis format
        ApplicationManager.getApplication().executeOnPooledThread {
            LOG.warn("Running format on file: ${file.path}")
            val result = LinthisExecutor.format(file.path, project.basePath, settings.linthisPath, settings.usePlugin)

            ApplicationManager.getApplication().invokeLater {
                if (result.success) {
                    LOG.warn("Format successful, reloading document")
                    // Refresh VirtualFile to detect changes on disk
                    file.refresh(false, false)
                    // Reload document content from disk into editor buffer
                    FileDocumentManager.getInstance().reloadFromDisk(document)

                    NotificationGroupManager.getInstance()
                        .getNotificationGroup("Linthis Notifications")
                        .createNotification(
                            "Format completed",
                            "File formatted successfully.",
                            NotificationType.INFORMATION
                        )
                        .notify(project)
                } else {
                    LOG.warn("Format failed: ${result.errorMessage}")
                    NotificationGroupManager.getInstance()
                        .getNotificationGroup("Linthis Notifications")
                        .createNotification(
                            "Format failed",
                            result.errorMessage ?: "Unknown error",
                            NotificationType.ERROR
                        )
                        .notify(project)
                }
            }
        }
    }

    override fun update(e: AnActionEvent) {
        val project = e.project
        val file = e.getData(CommonDataKeys.VIRTUAL_FILE)
        e.presentation.isEnabledAndVisible = project != null && file != null
    }

    override fun getActionUpdateThread(): ActionUpdateThread {
        return ActionUpdateThread.BGT
    }
}
