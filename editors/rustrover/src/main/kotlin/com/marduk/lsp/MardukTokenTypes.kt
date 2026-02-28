package com.marduk.lsp

import com.intellij.psi.tree.IElementType

object MardukTokenTypes {
    val KEYWORD     = IElementType("MKML_KEYWORD",     MardukLanguage)
    val WIDGET_NAME = IElementType("MKML_WIDGET_NAME", MardukLanguage)
    val IDENTIFIER  = IElementType("MKML_IDENTIFIER",  MardukLanguage)
    val PROP_KEY    = IElementType("MKML_PROP_KEY",    MardukLanguage)
    val STRING     = IElementType("MKML_STRING",     MardukLanguage)
    val COLOR      = IElementType("MKML_COLOR",      MardukLanguage)
    val NUMBER     = IElementType("MKML_NUMBER",     MardukLanguage)
    val COMMENT    = IElementType("MKML_COMMENT",    MardukLanguage)
    val BRACE      = IElementType("MKML_BRACE",      MardukLanguage)
    val COLON      = IElementType("MKML_COLON",      MardukLanguage)
}
