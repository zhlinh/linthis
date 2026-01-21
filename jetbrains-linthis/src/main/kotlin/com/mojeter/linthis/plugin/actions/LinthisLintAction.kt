// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT

package com.mojeter.linthis.plugin.actions

import com.intellij.openapi.actionSystem.ActionUpdateThread
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.actionSystem.CommonDataKeys
import com.intellij.openapi.fileEditor.FileDocumentManager
import com.intellij.openapi.project.DumbAware
import com.intellij.codeInsight.daemon.DaemonCodeAnalyzer

/**
 * Action to manually trigger linting on the current file.
 * Shortcut: Ctrl+Alt+Shift+L (Windows/Linux) or Cmd+Alt+Shift+L (Mac)
 *
 * This action forces a re-analysis of the current file, which will
 * trigger LSP4IJ to request fresh diagnostics from the language server.
 */
class LinthisLintAction : AnAction(), DumbAware {

    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project ?: return
        val editor = e.getData(CommonDataKeys.EDITOR) ?: return
        val psiFile = e.getData(CommonDataKeys.PSI_FILE) ?: return

        // Save the document first to ensure the server has the latest content
        FileDocumentManager.getInstance().saveDocument(editor.document)

        // Restart the daemon code analyzer to force re-analysis
        // This will trigger LSP4IJ's external annotator to request diagnostics
        DaemonCodeAnalyzer.getInstance(project).restart(psiFile)
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
