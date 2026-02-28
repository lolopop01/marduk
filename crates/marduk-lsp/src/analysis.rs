//! Heuristic source analysis for completion and hover.
//!
//! We deliberately avoid running the full parser here: files are always
//! syntactically incomplete at the cursor position.  Instead we use simple
//! text heuristics that work reliably in practice.

use tower_lsp::lsp_types::Position;

// ── Context kind ──────────────────────────────────────────────────────────────

/// What the cursor is positioned inside, used to drive completions.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Context {
    /// Top-level or depth-0: another widget name is expected.
    Widget,
    /// Inside a widget block, on a key position (before `:`).
    Property { widget: String },
    /// After the `:` on a `key: value` line.
    Value { widget: String, prop: String },
    /// Unknown (comment, inline content, etc.).
    Unknown,
}

// ── word_at ───────────────────────────────────────────────────────────────────

/// Extract the identifier (or partial identifier) that contains or immediately
/// precedes the cursor column.
///
/// Returns a sub-slice of the line, so the lifetime is tied to `text`.
pub fn word_at<'t>(text: &'t str, pos: &Position) -> Option<&'t str> {
    let line = text.lines().nth(pos.line as usize)?;
    let col = (pos.character as usize).min(line.len());

    let start = line[..col]
        .rfind(|c: char| !c.is_alphanumeric() && c != '_')
        .map(|i| i + 1)
        .unwrap_or(0);

    let end = col
        + line[col..]
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .unwrap_or(line.len() - col);

    if start < end {
        Some(&line[start..end])
    } else {
        None
    }
}

// ── find_enclosing_widget ─────────────────────────────────────────────────────

/// Walk backwards through `before` (text up to the cursor) counting braces to
/// find the nearest unclosed `{` block, then return the widget name that opened it.
pub fn find_enclosing_widget(before: &str) -> Option<String> {
    let mut depth: i32 = 0;

    for line in before.lines().rev() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            continue;
        }
        for ch in trimmed.chars().rev() {
            match ch {
                '}' => depth += 1,
                '{' => {
                    if depth == 0 {
                        return widget_name_on_line(trimmed);
                    }
                    depth -= 1;
                }
                _ => {}
            }
        }
    }
    None
}

fn widget_name_on_line(line: &str) -> Option<String> {
    let word = line.split_whitespace().next()?;
    // Widget names start with uppercase; skip keywords like `import`.
    if word.chars().next()?.is_uppercase() {
        Some(word.trim_end_matches('{').to_string())
    } else {
        None
    }
}

// ── completion_context ────────────────────────────────────────────────────────

/// Classify the cursor position for completion.
pub fn completion_context(text: &str, pos: &Position) -> Context {
    let line_idx = pos.line as usize;
    let col = pos.character as usize;

    let lines: Vec<&str> = text.lines().collect();
    let current_line = lines.get(line_idx).copied().unwrap_or("");
    let before_cursor = &current_line[..col.min(current_line.len())];
    let effective = strip_comment(before_cursor);

    // After a colon → we're completing a value.
    if let Some(colon_idx) = effective.rfind(':') {
        let prop = effective[..colon_idx].trim().to_string();
        let before = text_before(text, line_idx, col);
        let widget = find_enclosing_widget(&before).unwrap_or_default();
        return Context::Value { widget, prop };
    }

    let before = text_before(text, line_idx, col);
    let depth = brace_depth(&before);

    if depth == 0 {
        return Context::Widget;
    }

    // Inside a block: uppercase start → child widget, else → property key.
    let trimmed = effective.trim();
    if trimmed.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
        Context::Widget
    } else {
        let widget = find_enclosing_widget(&before).unwrap_or_default();
        Context::Property { widget }
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn strip_comment(s: &str) -> &str {
    s.find("//").map(|i| &s[..i]).unwrap_or(s)
}

fn brace_depth(text: &str) -> i32 {
    text.chars()
        .fold(0i32, |d, c| match c {
            '{' => d + 1,
            '}' => (d - 1).max(0),
            _ => d,
        })
}

/// Build the source text from the beginning of the file up to `(line, col)`.
fn text_before(text: &str, line_idx: usize, col: usize) -> String {
    let mut out = String::new();
    for (i, line) in text.lines().enumerate() {
        if i < line_idx {
            out.push_str(line);
            out.push('\n');
        } else if i == line_idx {
            let end = col.min(line.len());
            out.push_str(&line[..end]);
            break;
        } else {
            break;
        }
    }
    out
}
