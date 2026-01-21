// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT

package com.mojeter.linthis.plugin

import com.intellij.openapi.project.Project
import com.redhat.devtools.lsp4ij.client.LanguageClientImpl
import org.eclipse.lsp4j.PublishDiagnosticsParams

/**
 * Language client implementation for Linthis.
 *
 * This class handles communication between the IDE and the Linthis LSP server,
 * including receiving diagnostics and handling server notifications.
 *
 * The linthis LSP server includes tool information in the diagnostic's `source` field
 * (e.g., "linthis-ruff", "linthis-clippy"). We add this as a prefix to the message
 * for better visibility in inline editor hints.
 */
class LinthisLanguageClient(project: Project) : LanguageClientImpl(project) {

    override fun publishDiagnostics(params: PublishDiagnosticsParams) {
        // Add the source (e.g., "linthis-ruff") as a prefix to the message
        // for better visibility in inline editor hints
        params.diagnostics.forEach { diagnostic ->
            val source = diagnostic.source
            if (source != null && !diagnostic.message.startsWith("[$source]")) {
                diagnostic.message = "[$source] ${diagnostic.message}"
            }
        }

        super.publishDiagnostics(params)
    }

    override fun toString(): String {
        return "Linthis Language Client"
    }
}
