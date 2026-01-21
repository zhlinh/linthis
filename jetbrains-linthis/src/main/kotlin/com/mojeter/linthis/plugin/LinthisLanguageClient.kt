// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT

package com.mojeter.linthis.plugin

import com.intellij.openapi.project.Project
import com.redhat.devtools.lsp4ij.client.LanguageClientImpl

/**
 * Language client implementation for Linthis.
 *
 * This class handles communication between the IDE and the Linthis LSP server,
 * including receiving diagnostics and handling server notifications.
 */
class LinthisLanguageClient(project: Project) : LanguageClientImpl(project) {

    // The base LanguageClientImpl handles most functionality
    // Override methods here if custom behavior is needed

    override fun toString(): String {
        return "Linthis Language Client"
    }
}
