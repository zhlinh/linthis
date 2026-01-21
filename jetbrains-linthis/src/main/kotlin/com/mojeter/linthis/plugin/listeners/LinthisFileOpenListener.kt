// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT

package com.mojeter.linthis.plugin.listeners

import com.intellij.openapi.fileEditor.FileEditorManager
import com.intellij.openapi.fileEditor.FileEditorManagerListener
import com.intellij.openapi.vfs.VirtualFile
import com.mojeter.linthis.plugin.settings.LinthisSettings

/**
 * Listener for file open events.
 * Handles lint on open functionality.
 *
 * Note: LSP4IJ automatically sends textDocument/didOpen when a file is opened,
 * which triggers the LSP server to send diagnostics. This listener is mainly
 * for checking the settings and potentially triggering additional behavior.
 */
class LinthisFileOpenListener : FileEditorManagerListener {

    override fun fileOpened(source: FileEditorManager, file: VirtualFile) {
        val project = source.project
        val settings = LinthisSettings.getInstance(project)

        // LSP4IJ automatically handles didOpen notification when files are opened.
        // The lintOnOpen setting controls whether we want this behavior enabled.
        // Since LSP4IJ handles this automatically, we just check the setting here.
        if (!settings.lintOnOpen) {
            // If lint on open is disabled, we could potentially suppress diagnostics
            // but LSP4IJ doesn't easily support this per-file, so we leave it as-is
            // The setting is more for user documentation purposes
        }
    }
}
