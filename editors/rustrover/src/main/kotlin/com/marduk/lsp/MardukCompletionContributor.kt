package com.marduk.lsp

import com.intellij.codeInsight.completion.CompletionContributor
import com.intellij.codeInsight.completion.CompletionParameters
import com.intellij.codeInsight.completion.CompletionResultSet
import com.intellij.codeInsight.lookup.LookupElementBuilder
import com.intellij.icons.AllIcons

/**
 * Native completion contributor for .mkml files.
 *
 * Provides instant, synchronous completions (widget names, property keys, common
 * values) without waiting for the LSP server to respond. Context detection mirrors
 * the heuristic analysis in the LSP crate's analysis.rs.
 */
class MardukCompletionContributor : CompletionContributor() {

    override fun fillCompletionVariants(parameters: CompletionParameters, result: CompletionResultSet) {
        if (parameters.originalFile.language != MardukLanguage) return

        val text   = parameters.editor.document.text
        val offset = parameters.offset
        val before = text.substring(0, offset)

        // Current line content before the cursor, with comments stripped.
        val lineStart   = before.lastIndexOf('\n') + 1
        val currentLine = stripAllComments(before.substring(lineStart)).trimStart()

        val colonIdx = currentLine.indexOf(':')
        when {
            // After a colon → value completions.
            colonIdx >= 0 -> {
                val prop   = currentLine.substring(0, colonIdx).trim()
                val widget = findEnclosingWidget(before)
                addValueCompletions(result, widget, prop)
            }
            // Depth 0 or current line starts with uppercase → widget names.
            braceDepth(stripAllComments(before)) == 0 ||
            currentLine.firstOrNull()?.isUpperCase() == true -> {
                addWidgetCompletions(result)
            }
            // Inside a block, lowercase start → property keys.
            else -> {
                val widget = findEnclosingWidget(before)
                addPropertyCompletions(result, widget)
            }
        }
    }

    // ── completion item builders ──────────────────────────────────────────────

    private fun addWidgetCompletions(result: CompletionResultSet) {
        for (name in MardukKnowledge.WIDGETS) {
            result.addElement(
                LookupElementBuilder.create(name)
                    .withIcon(AllIcons.Nodes.Class)
                    .withTypeText("widget")
                    .bold()
            )
        }
    }

    private fun addPropertyCompletions(result: CompletionResultSet, widget: String?) {
        for (prop in MardukKnowledge.propsFor(widget ?: "")) {
            result.addElement(
                LookupElementBuilder.create(prop)
                    .withIcon(AllIcons.Nodes.Field)
                    .withTypeText("property")
                    .withInsertHandler { ctx, _ ->
                        val doc = ctx.document
                        val tail = ctx.tailOffset
                        // Only append ": " if not already present.
                        if (tail >= doc.textLength || doc.text[tail] != ':') {
                            doc.insertString(tail, ": ")
                            ctx.editor.caretModel.moveToOffset(tail + 2)
                        }
                    }
            )
        }
    }

    private fun addValueCompletions(result: CompletionResultSet, widget: String?, prop: String) {
        when {
            prop.endsWith("color") || prop == "bg" || prop == "fg"
            || prop == "hover_bg" || prop == "press_bg"
            || prop == "text_color" || prop == "border_color" -> {
                result.addElement(
                    LookupElementBuilder.create("#rrggbbaa")
                        .withIcon(AllIcons.Nodes.DataTables)
                        .withTypeText("color literal")
                )
            }
            prop == "align" -> {
                for (v in listOf("start", "center", "end")) {
                    result.addElement(LookupElementBuilder.create(v).withTypeText("align"))
                }
            }
            prop == "value" || prop == "on_change" || prop == "on_click"
            || prop == "on_drag" || prop == "on_scroll" -> {
                // Event/identifier values: no specific completions — the LSP provides these.
            }
            else -> Unit
        }
    }

    // ── text analysis (mirrors analysis.rs) ──────────────────────────────────

    private fun stripAllComments(text: String): String {
        val sb = StringBuilder(text.length)
        var i = 0
        while (i < text.length) {
            when {
                text.startsWith("//", i) -> {
                    while (i < text.length && text[i] != '\n') { sb.append(' '); i++ }
                }
                text.startsWith("/*", i) -> {
                    sb.append("  "); i += 2
                    while (i < text.length) {
                        if (text.startsWith("*/", i)) { sb.append("  "); i += 2; break }
                        sb.append(if (text[i] == '\n') '\n' else ' ')
                        i++
                    }
                }
                else -> { sb.append(text[i]); i++ }
            }
        }
        return sb.toString()
    }

    private fun braceDepth(text: String): Int =
        text.fold(0) { d, c -> when (c) { '{' -> d + 1; '}' -> (d - 1).coerceAtLeast(0); else -> d } }

    private fun findEnclosingWidget(before: String): String? {
        val stripped = stripAllComments(before)
        var depth = 0
        for (line in stripped.lines().reversed()) {
            val trimmed = line.trim()
            for (c in trimmed.reversed()) {
                when (c) {
                    '}' -> depth++
                    '{' -> {
                        if (depth == 0) {
                            val word = trimmed.split(Regex("\\s+")).firstOrNull() ?: return null
                            return if (word.firstOrNull()?.isUpperCase() == true)
                                word.trimEnd('{') else null
                        }
                        depth--
                    }
                }
            }
        }
        return null
    }
}
