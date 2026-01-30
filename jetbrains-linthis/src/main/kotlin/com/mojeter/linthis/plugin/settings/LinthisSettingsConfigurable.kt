// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT

package com.mojeter.linthis.plugin.settings

import com.intellij.openapi.fileChooser.FileChooserDescriptorFactory
import com.intellij.openapi.options.Configurable
import com.intellij.openapi.project.Project
import com.intellij.openapi.ui.TextFieldWithBrowseButton
import com.intellij.ui.components.JBCheckBox
import com.intellij.ui.components.JBLabel
import com.intellij.ui.components.JBTextField
import com.intellij.util.ui.FormBuilder
import javax.swing.JComponent
import javax.swing.JPanel

/**
 * Configurable for Linthis plugin settings.
 * Provides UI for configuring lint/format options.
 */
class LinthisSettingsConfigurable(private val project: Project) : Configurable {

    private var settingsPanel: JPanel? = null
    private var lintOnOpenCheckbox: JBCheckBox? = null
    private var lintOnSaveCheckbox: JBCheckBox? = null
    private var formatOnSaveCheckbox: JBCheckBox? = null
    private var linthisPathField: TextFieldWithBrowseButton? = null
    private var usePluginField: JBTextField? = null
    private var additionalArgsField: JBTextField? = null

    override fun getDisplayName(): String = "Linthis"

    override fun createComponent(): JComponent {
        lintOnOpenCheckbox = JBCheckBox("Lint on file open")
        lintOnSaveCheckbox = JBCheckBox("Lint on save")
        formatOnSaveCheckbox = JBCheckBox("Format on save")

        linthisPathField = TextFieldWithBrowseButton().apply {
            addBrowseFolderListener(
                "Select Linthis Executable",
                "Select the path to the linthis executable",
                project,
                FileChooserDescriptorFactory.createSingleFileDescriptor()
            )
        }

        additionalArgsField = JBTextField()
        usePluginField = JBTextField()

        settingsPanel = FormBuilder.createFormBuilder()
            .addComponent(JBLabel("<html><b>Behavior</b></html>"))
            .addComponent(lintOnOpenCheckbox!!)
            .addComponent(lintOnSaveCheckbox!!)
            .addComponent(formatOnSaveCheckbox!!)
            .addSeparator()
            .addComponent(JBLabel("<html><b>Executable</b></html>"))
            .addLabeledComponent(
                JBLabel("Linthis path (leave empty for auto-detect):"),
                linthisPathField!!
            )
            .addSeparator()
            .addComponent(JBLabel("<html><b>Plugin</b></html>"))
            .addLabeledComponent(
                JBLabel("<html>Use plugin (e.g., https://github.com/zhlinh/linthis-plugin-template):</html>"),
                usePluginField!!
            )
            .addSeparator()
            .addComponent(JBLabel("<html><b>Advanced</b></html>"))
            .addLabeledComponent(
                JBLabel("Additional arguments:"),
                additionalArgsField!!
            )
            .addComponentFillVertically(JPanel(), 0)
            .panel

        return settingsPanel!!
    }

    override fun isModified(): Boolean {
        val settings = LinthisSettings.getInstance(project)
        return lintOnOpenCheckbox?.isSelected != settings.lintOnOpen ||
               lintOnSaveCheckbox?.isSelected != settings.lintOnSave ||
               formatOnSaveCheckbox?.isSelected != settings.formatOnSave ||
               linthisPathField?.text != settings.linthisPath ||
               usePluginField?.text != settings.usePlugin ||
               additionalArgsField?.text != settings.additionalArgs
    }

    override fun apply() {
        val settings = LinthisSettings.getInstance(project)
        settings.lintOnOpen = lintOnOpenCheckbox?.isSelected ?: true
        settings.lintOnSave = lintOnSaveCheckbox?.isSelected ?: true
        settings.formatOnSave = formatOnSaveCheckbox?.isSelected ?: false
        settings.linthisPath = linthisPathField?.text ?: ""
        settings.usePlugin = usePluginField?.text ?: ""
        settings.additionalArgs = additionalArgsField?.text ?: ""
    }

    override fun reset() {
        val settings = LinthisSettings.getInstance(project)
        lintOnOpenCheckbox?.isSelected = settings.lintOnOpen
        lintOnSaveCheckbox?.isSelected = settings.lintOnSave
        formatOnSaveCheckbox?.isSelected = settings.formatOnSave
        linthisPathField?.text = settings.linthisPath
        usePluginField?.text = settings.usePlugin
        additionalArgsField?.text = settings.additionalArgs
    }

    override fun disposeUIResources() {
        settingsPanel = null
        lintOnOpenCheckbox = null
        lintOnSaveCheckbox = null
        formatOnSaveCheckbox = null
        linthisPathField = null
        usePluginField = null
        additionalArgsField = null
    }
}
