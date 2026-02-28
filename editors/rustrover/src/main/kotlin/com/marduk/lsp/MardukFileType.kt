package com.marduk.lsp

import com.intellij.lang.Language
import com.intellij.openapi.fileTypes.LanguageFileType
import javax.swing.Icon

// A minimal Language token — required by LanguageFileType.
// No grammar or parser is provided here; the LSP server handles all semantics.
object MardukLanguage : Language("Marduk")

object MardukFileType : LanguageFileType(MardukLanguage) {
    override fun getName(): String        = "Marduk Markup Language"
    override fun getDescription(): String = "Marduk Markup Language (.mkml)"
    override fun getDefaultExtension(): String = "mkml"
    override fun getIcon(): Icon?         = null
}
