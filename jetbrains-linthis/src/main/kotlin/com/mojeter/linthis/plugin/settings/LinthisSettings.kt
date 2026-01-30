// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT

package com.mojeter.linthis.plugin.settings

import com.intellij.openapi.components.*
import com.intellij.openapi.project.Project
import com.intellij.util.xmlb.XmlSerializerUtil

/**
 * Persistent settings for the Linthis plugin.
 * Settings are stored per-project.
 */
@State(
    name = "LinthisSettings",
    storages = [Storage("linthis.xml")]
)
@Service(Service.Level.PROJECT)
class LinthisSettings : PersistentStateComponent<LinthisSettings.State> {

    private var myState = State()

    class State {
        var lintOnOpen: Boolean = true
        var lintOnSave: Boolean = true
        var formatOnSave: Boolean = false
        var linthisPath: String = ""
        var usePlugin: String = ""
        var additionalArgs: String = ""
    }

    override fun getState(): State = myState

    override fun loadState(state: State) {
        XmlSerializerUtil.copyBean(state, myState)
    }

    var lintOnOpen: Boolean
        get() = myState.lintOnOpen
        set(value) { myState.lintOnOpen = value }

    var lintOnSave: Boolean
        get() = myState.lintOnSave
        set(value) { myState.lintOnSave = value }

    var formatOnSave: Boolean
        get() = myState.formatOnSave
        set(value) { myState.formatOnSave = value }

    var linthisPath: String
        get() = myState.linthisPath
        set(value) { myState.linthisPath = value }

    var usePlugin: String
        get() = myState.usePlugin
        set(value) { myState.usePlugin = value }

    var additionalArgs: String
        get() = myState.additionalArgs
        set(value) { myState.additionalArgs = value }

    companion object {
        fun getInstance(project: Project): LinthisSettings {
            return project.getService(LinthisSettings::class.java)
        }
    }
}
