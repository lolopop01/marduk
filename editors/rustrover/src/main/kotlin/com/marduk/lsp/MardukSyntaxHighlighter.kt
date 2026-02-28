package com.marduk.lsp

import com.intellij.lexer.Lexer
import com.intellij.openapi.editor.DefaultLanguageHighlighterColors
import com.intellij.openapi.editor.colors.TextAttributesKey
import com.intellij.openapi.editor.colors.TextAttributesKey.createTextAttributesKey
import com.intellij.openapi.fileTypes.SyntaxHighlighterBase
import com.intellij.psi.tree.IElementType

class MardukSyntaxHighlighter : SyntaxHighlighterBase() {

    companion object {
        val KEYWORD    = createTextAttributesKey("MKML_KEYWORD",    DefaultLanguageHighlighterColors.KEYWORD)
        val COMMENT    = createTextAttributesKey("MKML_COMMENT",    DefaultLanguageHighlighterColors.LINE_COMMENT)
        val STRING     = createTextAttributesKey("MKML_STRING",     DefaultLanguageHighlighterColors.STRING)
        // Color literals get their own key so users can style them independently;
        // they fall back to CONSTANT (typically cyan/teal in dark themes).
        val COLOR      = createTextAttributesKey("MKML_COLOR",      DefaultLanguageHighlighterColors.CONSTANT)
        val NUMBER     = createTextAttributesKey("MKML_NUMBER",     DefaultLanguageHighlighterColors.NUMBER)
        val BRACE      = createTextAttributesKey("MKML_BRACE",      DefaultLanguageHighlighterColors.BRACES)
        val COLON      = createTextAttributesKey("MKML_COLON",      DefaultLanguageHighlighterColors.OPERATION_SIGN)
        // Property keys (e.g. `bg`, `gap`, `on_click`) styled like struct fields.
        val PROP_KEY     = createTextAttributesKey("MKML_PROP_KEY",     DefaultLanguageHighlighterColors.INSTANCE_FIELD)
        // Widget / import alias names (PascalCase) styled like class names.
        val WIDGET_NAME  = createTextAttributesKey("MKML_WIDGET_NAME",  DefaultLanguageHighlighterColors.CLASS_NAME)
    }

    override fun getHighlightingLexer(): Lexer = MardukLexer()

    override fun getTokenHighlights(type: IElementType): Array<TextAttributesKey> =
        when (type) {
            MardukTokenTypes.KEYWORD  -> pack(KEYWORD)
            MardukTokenTypes.COMMENT  -> pack(COMMENT)
            MardukTokenTypes.STRING   -> pack(STRING)
            MardukTokenTypes.COLOR    -> pack(COLOR)
            MardukTokenTypes.NUMBER   -> pack(NUMBER)
            MardukTokenTypes.BRACE    -> pack(BRACE)
            MardukTokenTypes.COLON    -> pack(COLON)
            MardukTokenTypes.PROP_KEY    -> pack(PROP_KEY)
            MardukTokenTypes.WIDGET_NAME -> pack(WIDGET_NAME)
            else -> emptyArray()
        }
}
