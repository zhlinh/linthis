// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT

package com.mojeter.linthis.plugin

import com.intellij.openapi.project.Project
import com.redhat.devtools.lsp4ij.LanguageServerFactory
import com.redhat.devtools.lsp4ij.client.LanguageClientImpl
import com.redhat.devtools.lsp4ij.server.StreamConnectionProvider

/**
 * Factory for creating Linthis Language Server instances.
 *
 * This factory is registered in plugin.xml and is responsible for:
 * - Creating the connection provider that starts the linthis LSP server
 * - Creating the language client for communication
 */
class LinthisLanguageServerFactory : LanguageServerFactory {

    override fun createConnectionProvider(project: Project): StreamConnectionProvider {
        return LinthisStreamConnectionProvider(project)
    }

    override fun createLanguageClient(project: Project): LanguageClientImpl {
        return LinthisLanguageClient(project)
    }
}
