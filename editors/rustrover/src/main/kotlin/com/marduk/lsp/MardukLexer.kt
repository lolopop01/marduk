package com.marduk.lsp

import com.intellij.lexer.LexerBase
import com.intellij.psi.TokenType
import com.intellij.psi.tree.IElementType

/**
 * Hand-written lexer for .mkml files.
 *
 * Token categories:
 *  - KEYWORD     → import | as
 *  - WIDGET_NAME → PascalCase identifier (not followed by ':')
 *  - PROP_KEY    → identifier immediately followed (on the same line) by ':'
 *  - IDENTIFIER  → event names, enum values, etc.
 *  - STRING      → "…"  (backslash escapes honoured)
 *  - COLOR       → #rrggbbaa  (# followed by hex digits)
 *  - NUMBER      → integer or float, optionally negative
 *  - COMMENT     → // … end-of-line  or  /* … */
 *  - BRACE       → { }
 *  - COLON       → :
 */
class MardukLexer : LexerBase() {

    private var buffer: CharSequence = ""
    private var bufEnd   = 0
    private var tokStart = 0
    private var tokEnd   = 0
    private var tokType: IElementType? = null

    override fun start(buffer: CharSequence, startOffset: Int, endOffset: Int, initialState: Int) {
        this.buffer   = buffer
        this.bufEnd   = endOffset
        this.tokStart = startOffset
        this.tokEnd   = startOffset
        advance()
    }

    override fun getState(): Int                = 0
    override fun getTokenType(): IElementType?  = tokType
    override fun getTokenStart(): Int           = tokStart
    override fun getTokenEnd(): Int             = tokEnd
    override fun getBufferSequence(): CharSequence = buffer
    override fun getBufferEnd(): Int            = bufEnd

    override fun advance() {
        tokStart = tokEnd
        tokType  = if (tokStart < bufEnd) scan() else null
    }

    // ── scanner ─────────────────────────────────────────────────────────────

    private fun scan(): IElementType {
        val pos = tokStart
        val c   = buffer[pos]

        // Whitespace
        if (c.isWhitespace()) {
            tokEnd = pos + 1
            while (tokEnd < bufEnd && buffer[tokEnd].isWhitespace()) tokEnd++
            return TokenType.WHITE_SPACE
        }

        // Line comment  //
        if (c == '/' && pos + 1 < bufEnd && buffer[pos + 1] == '/') {
            tokEnd = pos + 2
            while (tokEnd < bufEnd && buffer[tokEnd] != '\n') tokEnd++
            return MardukTokenTypes.COMMENT
        }

        // Block comment  /* … */
        if (c == '/' && pos + 1 < bufEnd && buffer[pos + 1] == '*') {
            tokEnd = pos + 2
            while (tokEnd < bufEnd) {
                if (buffer[tokEnd] == '*' && tokEnd + 1 < bufEnd && buffer[tokEnd + 1] == '/') {
                    tokEnd += 2
                    break
                }
                tokEnd++
            }
            return MardukTokenTypes.COMMENT
        }

        // String literal  "…"
        if (c == '"') {
            tokEnd = pos + 1
            while (tokEnd < bufEnd && buffer[tokEnd] != '"') {
                if (buffer[tokEnd] == '\\') tokEnd++ // skip escaped char
                tokEnd++
            }
            if (tokEnd < bufEnd) tokEnd++ // consume closing "
            return MardukTokenTypes.STRING
        }

        // Color literal  #rrggbbaa
        if (c == '#') {
            tokEnd = pos + 1
            while (tokEnd < bufEnd && isHex(buffer[tokEnd])) tokEnd++
            return MardukTokenTypes.COLOR
        }

        // Number  (optional leading -, digits, optional dot)
        if (c.isDigit() || (c == '-' && pos + 1 < bufEnd && buffer[pos + 1].isDigit())) {
            tokEnd = pos + 1
            while (tokEnd < bufEnd && (buffer[tokEnd].isDigit() || buffer[tokEnd] == '.')) tokEnd++
            return MardukTokenTypes.NUMBER
        }

        // Identifier, keyword, widget name, or property key
        if (c.isLetter() || c == '_') {
            tokEnd = pos + 1
            while (tokEnd < bufEnd && (buffer[tokEnd].isLetterOrDigit() || buffer[tokEnd] == '_')) tokEnd++
            val word = buffer.substring(pos, tokEnd)
            return when {
                word == "import" || word == "as" -> MardukTokenTypes.KEYWORD
                nextNonSpaceOnLine(tokEnd) == ':' -> MardukTokenTypes.PROP_KEY
                c.isUpperCase() -> MardukTokenTypes.WIDGET_NAME   // PascalCase → widget/alias
                else -> MardukTokenTypes.IDENTIFIER
            }
        }

        // Braces
        if (c == '{' || c == '}') {
            tokEnd = pos + 1
            return MardukTokenTypes.BRACE
        }

        // Colon
        if (c == ':') {
            tokEnd = pos + 1
            return MardukTokenTypes.COLON
        }

        tokEnd = pos + 1
        return TokenType.BAD_CHARACTER
    }

    // ── helpers ──────────────────────────────────────────────────────────────

    /** Next non-space character on the same line, or null if newline/EOF comes first. */
    private fun nextNonSpaceOnLine(from: Int): Char? {
        var i = from
        while (i < bufEnd && (buffer[i] == ' ' || buffer[i] == '\t')) i++
        return if (i < bufEnd && buffer[i] != '\n' && buffer[i] != '\r') buffer[i] else null
    }

    private fun isHex(c: Char) = c in '0'..'9' || c in 'a'..'f' || c in 'A'..'F'
}
