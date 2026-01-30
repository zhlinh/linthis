// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT

package com.mojeter.linthis.plugin.listeners

import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.diagnostic.Logger
import com.intellij.openapi.editor.Document
import com.intellij.openapi.fileEditor.FileDocumentManager
import com.intellij.openapi.fileEditor.FileDocumentManagerListener
import com.intellij.openapi.project.ProjectLocator
import com.intellij.openapi.project.ProjectManager
import com.mojeter.linthis.plugin.settings.LinthisSettings
import com.mojeter.linthis.plugin.util.LinthisExecutor

/**
 * Listener for document save events.
 * Handles format on save functionality using linthis CLI.
 *
 * Note: Lint on save is automatically handled by LSP4IJ through didSave notification.
 */
class LinthisDocumentListener : FileDocumentManagerListener {

    companion object {
        private val LOG = Logger.getInstance(LinthisDocumentListener::class.java)
    }

    override fun beforeDocumentSaving(document: Document) {
        LOG.warn("beforeDocumentSaving called")

        val file = FileDocumentManager.getInstance().getFile(document)
        if (file == null) {
            LOG.warn("beforeDocumentSaving: file is null, returning")
            return
        }
        LOG.warn("beforeDocumentSaving: file = ${file.path}")

        // Find the project for this file using ProjectLocator (more reliable)
        var project = ProjectLocator.getInstance().guessProjectForFile(file)
        LOG.warn("beforeDocumentSaving: ProjectLocator returned project = ${project?.name}, basePath = ${project?.basePath}")

        // Fallback to checking open projects if ProjectLocator fails
        if (project == null) {
            val openProjects = ProjectManager.getInstance().openProjects
            LOG.warn("beforeDocumentSaving: fallback - openProjects count = ${openProjects.size}")

            project = openProjects.firstOrNull { proj ->
                val basePath = proj.basePath
                LOG.warn("beforeDocumentSaving: checking project basePath = $basePath")
                basePath != null && file.path.startsWith(basePath)
            }
        }

        // If still no project, try to use any open project (for sandbox testing)
        if (project == null) {
            val openProjects = ProjectManager.getInstance().openProjects
            project = openProjects.firstOrNull { !it.isDefault }
            LOG.warn("beforeDocumentSaving: using first non-default project = ${project?.name}")
        }

        if (project == null) {
            LOG.warn("beforeDocumentSaving: no project found for ${file.path}, returning")
            return
        }
        LOG.warn("beforeDocumentSaving: using project = ${project.name}")

        val settings = LinthisSettings.getInstance(project)
        LOG.warn("beforeDocumentSaving: formatOnSave = ${settings.formatOnSave}")

        // Format on save: schedule format AFTER save completes
        if (settings.formatOnSave) {
            LOG.warn("Format on save scheduled for: ${file.path}")

            // Schedule format to run after save completes
            ApplicationManager.getApplication().invokeLater {
                // Run format in background thread
                ApplicationManager.getApplication().executeOnPooledThread {
                    LOG.warn("Running format on save for: ${file.path}")

                    // Format the file in-place (file is already saved to disk)
                    val result = LinthisExecutor.format(file.path, project.basePath, settings.linthisPath, settings.usePlugin)

                    if (result.success) {
                        LOG.warn("Format on save successful, reloading document")
                        // Reload document from disk on EDT
                        ApplicationManager.getApplication().invokeLater {
                            file.refresh(false, false)
                            FileDocumentManager.getInstance().reloadFromDisk(document)
                        }
                    } else {
                        LOG.warn("Format on save failed: ${result.errorMessage}")
                    }
                }
            }
        }

        // Lint on save is automatically handled by LSP4IJ through didSave notification
    }
}
