use memchr::memchr;

use oxc_allocator::Allocator;
use oxc_parser::{ParseOptions, Parser};
use oxc_span::SourceType;

use crate::{
    firstpass::FirstPass,
    parse::{Item, ItemBody},
};

/// Result of trying to parse an ESM block with oxc.
pub(crate) enum EsmParseResult {
    Complete,
    Incomplete,
    Error,
}

/// Try to parse an ESM block to check completeness.
///
/// Accepts a reusable allocator to avoid repeated allocation in the
/// blank-line retry loop.
pub(crate) fn try_parse_esm(value: &str, allocator: &mut Allocator) -> EsmParseResult {
    allocator.reset();
    let source_type = SourceType::mjs().with_jsx(true);
    let source = allocator.alloc_str(value);
    let ret = Parser::new(allocator, source, source_type)
        .with_options(ParseOptions::default())
        .parse();

    if ret.errors.is_empty() {
        return EsmParseResult::Complete;
    }

    let error = &ret.errors[0];
    let error_offset = error
        .labels
        .as_ref()
        .and_then(|labels| labels.first().map(|l| l.offset()))
        .unwrap_or(value.len());

    if error_offset >= value.len() {
        EsmParseResult::Incomplete
    } else {
        EsmParseResult::Error
    }
}

/// Keywords after which `/` starts a regex literal, not division.
const REGEX_KEYWORDS: &[&[u8]] = &[
    b"await",
    b"case",
    b"delete",
    b"in",
    b"instanceof",
    b"new",
    b"of",
    b"return",
    b"throw",
    b"typeof",
    b"void",
    b"yield",
];

/// Determine whether `/` at `pos` is the start of a regex literal.
fn slash_is_regex(bytes: &[u8], pos: usize) -> bool {
    let mut i = pos;
    while i > 0 {
        i -= 1;
        match bytes[i] {
            b' ' | b'\t' | b'\n' | b'\r' => continue,
            b')' | b']' => return false,
            b'"' | b'\'' | b'`' => return false,
            b'+' if i > 0 && bytes[i - 1] == b'+' => return false,
            b'-' if i > 0 && bytes[i - 1] == b'-' => return false,
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'$' => {
                let end = i + 1;
                while i > 0
                    && (bytes[i - 1].is_ascii_alphanumeric()
                        || bytes[i - 1] == b'_'
                        || bytes[i - 1] == b'$')
                {
                    i -= 1;
                }
                let word = &bytes[i..end];
                let is_keyword_boundary = i == 0
                    || (!bytes[i - 1].is_ascii_alphanumeric()
                        && bytes[i - 1] != b'_'
                        && bytes[i - 1] != b'$');
                if is_keyword_boundary && REGEX_KEYWORDS.contains(&word) {
                    return true;
                }
                return false;
            }
            _ => return true,
        }
    }
    true
}

/// Scan a regex literal starting at `/`, returning the offset past the flags.
fn scan_regex(bytes: &[u8], start: usize) -> usize {
    let mut ix = start + 1;
    while ix < bytes.len() {
        match bytes[ix] {
            b'/' => {
                ix += 1;
                while ix < bytes.len() && bytes[ix].is_ascii_alphanumeric() {
                    ix += 1;
                }
                return ix;
            }
            b'\\' => ix += 2,
            b'[' => {
                ix += 1;
                while ix < bytes.len() && bytes[ix] != b']' {
                    if bytes[ix] == b'\\' {
                        ix += 1;
                    }
                    ix += 1;
                }
                if ix < bytes.len() {
                    ix += 1;
                }
            }
            b'\n' | b'\r' => return ix,
            _ => ix += 1,
        }
    }
    ix
}

/// Is the byte at `ix` a line ending followed by a blank line (or EOF)?
///
/// Used to cut the expression scan at block boundaries, so an unclosed `{`
/// can't silently consume subsequent blocks.
fn is_blank_line_next(bytes: &[u8], ix: usize) -> bool {
    let mut j = ix + 1;
    if bytes[ix] == b'\r' && j < bytes.len() && bytes[j] == b'\n' {
        j += 1;
    }
    let mut k = j;
    while k < bytes.len() && (bytes[k] == b' ' || bytes[k] == b'\t') {
        k += 1;
    }
    k >= bytes.len() || bytes[k] == b'\n' || bytes[k] == b'\r'
}

/// Scan an MDX expression `{...}`, finding the matching closing `}`.
///
/// Uses a lightweight JS lexer that properly handles strings, comments,
/// template literals, and regex literals. MDX expressions are rarely complex, so this should cover the vast majority of cases without needing a parser. I kinda wish oxc's exposed his lexer, though.
///
/// When `inline` is true, blank lines abort the scan — inline (text)
/// expressions must stay within a single paragraph, so the parser can't let
/// an unclosed `{` silently consume later blocks. Flow (block) expressions
/// are allowed to span blank lines.
///
/// Returns the byte offset past the closing `}`, or `None` if unclosed.
fn scan_mdx_expression_end(bytes: &[u8], inline: bool) -> Option<usize> {
    if bytes.is_empty() || bytes[0] != b'{' {
        return None;
    }

    let mut ix = 1;
    let mut depth: usize = 1;

    while ix < bytes.len() && depth > 0 {
        match bytes[ix] {
            b'\n' | b'\r' => {
                if inline && is_blank_line_next(bytes, ix) {
                    return None;
                }
                ix += 1;
            }
            b'{' => {
                depth += 1;
                ix += 1;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(ix + 1);
                }
                ix += 1;
            }
            // String literals (cannot span lines in JS)
            b'"' | b'\'' => {
                let quote = bytes[ix];
                ix += 1;
                while ix < bytes.len()
                    && bytes[ix] != quote
                    && bytes[ix] != b'\n'
                    && bytes[ix] != b'\r'
                {
                    if bytes[ix] == b'\\' {
                        ix += 1;
                    }
                    ix += 1;
                }
                if ix < bytes.len() && bytes[ix] == quote {
                    ix += 1;
                }
            }
            // Template literals with ${} nesting.
            b'`' => {
                ix += 1;
                let mut template_depth: usize = 0;
                while ix < bytes.len() {
                    match bytes[ix] {
                        b'`' if template_depth == 0 => {
                            ix += 1;
                            break;
                        }
                        b'\\' => {
                            ix += 2;
                            continue;
                        }
                        b'$' if ix + 1 < bytes.len() && bytes[ix + 1] == b'{' => {
                            template_depth += 1;
                            ix += 2;
                            continue;
                        }
                        b'{' if template_depth > 0 => template_depth += 1,
                        b'}' if template_depth > 0 => template_depth -= 1,
                        b'\n' | b'\r' if inline => {
                            if is_blank_line_next(bytes, ix) {
                                return None;
                            }
                        }
                        _ => {}
                    }
                    ix += 1;
                }
            }
            // Line comment
            b'/' if ix + 1 < bytes.len() && bytes[ix + 1] == b'/' => {
                ix += 2;
                while ix < bytes.len() && bytes[ix] != b'\n' {
                    ix += 1;
                }
            }
            // Block comment.
            b'/' if ix + 1 < bytes.len() && bytes[ix + 1] == b'*' => {
                ix += 2;
                while ix + 1 < bytes.len() {
                    if bytes[ix] == b'*' && bytes[ix + 1] == b'/' {
                        ix += 2;
                        break;
                    }
                    if inline
                        && (bytes[ix] == b'\n' || bytes[ix] == b'\r')
                        && is_blank_line_next(bytes, ix)
                    {
                        return None;
                    }
                    ix += 1;
                }
            }
            // Regex literal
            b'/' if slash_is_regex(bytes, ix) => {
                ix = scan_regex(bytes, ix);
            }
            // JSX tags — skip `<tag ...>` and `</tag>` so their `/` and `{}`
            // don't interfere with brace/regex tracking.
            b'<' if ix + 1 < bytes.len()
                && (bytes[ix + 1].is_ascii_alphabetic()
                    || bytes[ix + 1] == b'_'
                    || bytes[ix + 1] == b'$'
                    || bytes[ix + 1] == b'/'
                    || bytes[ix + 1] == b'>') =>
            {
                if let Some(end) = scan_mdx_jsx_tag_end(&bytes[ix..]) {
                    ix += end;
                } else {
                    ix += 1;
                }
            }
            _ => ix += 1,
        }
    }
    None
}

/// Check if from `start` to the next newline (or EOF) there are only spaces/tabs.
fn is_only_whitespace_to_eol(bytes: &[u8]) -> bool {
    for &b in bytes {
        match b {
            b' ' | b'\t' => continue,
            b'\n' | b'\r' => return true,
            _ => return false,
        }
    }
    true // EOF counts
}

/// Scan to the end of a line, returning offset past the newline.
fn scan_to_line_end(bytes: &[u8], start: usize) -> Option<usize> {
    let eol = memchr(b'\n', &bytes[start..])
        .map(|i| start + i + 1)
        .unwrap_or(bytes.len());
    Some(eol)
}

/// Scan a JSX tag from `<` to `>` or `/>`, returning the byte offset
/// immediately after the closing `>`. Does NOT scan to EOL.
fn scan_mdx_jsx_tag_end(bytes: &[u8]) -> Option<usize> {
    let mut ix = 1; // skip `<`

    // Skip `/` for closing tags
    if ix < bytes.len() && bytes[ix] == b'/' {
        ix += 1;
    }

    // Fragment `<>` / `</>`
    if ix < bytes.len() && bytes[ix] == b'>' {
        return Some(ix + 1);
    }

    // Validate tag name: must start with letter, `_`, or `$`
    if ix >= bytes.len() {
        return None;
    }
    let first = bytes[ix];
    if !first.is_ascii_alphabetic() && first != b'_' && first != b'$' && first < 0x80 {
        return None;
    }
    ix += 1;

    // Scan tag name body
    while ix < bytes.len() {
        match bytes[ix] {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'$' => ix += 1,
            b':' => {
                ix += 1;
                if ix >= bytes.len() {
                    return None;
                }
                let ch = bytes[ix];
                if !ch.is_ascii_alphabetic() && ch != b'_' && ch != b'$' && ch < 0x80 {
                    return None;
                }
                ix += 1;
            }
            0x80.. => ix += 1,
            _ => break,
        }
    }

    // After tag name: must be whitespace, `>`, `/`, or `{`
    if ix < bytes.len() {
        match bytes[ix] {
            b'>' | b'/' | b' ' | b'\t' | b'\n' | b'\r' | b'{' => {}
            _ => return None,
        }
    }

    while ix < bytes.len() {
        match bytes[ix] {
            b'>' => return Some(ix + 1),
            b'/' if ix + 1 < bytes.len() && bytes[ix + 1] == b'>' => {
                return Some(ix + 2);
            }
            b'{' => {
                // Attribute expression — use lexer to find the matching `}`.
                // JSX attributes live inside a tag, so blank lines abort.
                let expr_len = scan_mdx_expression_end(&bytes[ix..], true)?;
                ix += expr_len;
            }
            b'"' => {
                ix += 1;
                while ix < bytes.len() && bytes[ix] != b'"' {
                    if bytes[ix] == b'\\' {
                        ix += 1;
                    }
                    ix += 1;
                }
                if ix < bytes.len() {
                    ix += 1;
                }
            }
            b'\'' => {
                ix += 1;
                while ix < bytes.len() && bytes[ix] != b'\'' {
                    if bytes[ix] == b'\\' {
                        ix += 1;
                    }
                    ix += 1;
                }
                if ix < bytes.len() {
                    ix += 1;
                }
            }
            b'\n' | b'\r' => {
                ix += 1;
                if ix < bytes.len() && bytes[ix - 1] == b'\r' && bytes[ix] == b'\n' {
                    ix += 1;
                }
            }
            _ => ix += 1,
        }
    }
    None // Unclosed tag
}

/// Scan for an MDX ESM block (`import ...` or `export ...`).
///
/// Greedily consumes all non-blank continuation lines, matching the reference
/// mdxjs behavior. The actual completeness check is done later by the firstpass
/// using oxc when a blank line is encountered.
///
/// Returns the byte offset past the end of the block (including trailing newline).
pub(crate) fn scan_mdx_esm(bytes: &[u8]) -> Option<usize> {
    let is_import = bytes.starts_with(b"import ")
        || bytes.starts_with(b"import\t")
        || bytes.starts_with(b"import{");
    let is_export = bytes.starts_with(b"export ")
        || bytes.starts_with(b"export\t")
        || bytes.starts_with(b"export{")
        || bytes.starts_with(b"export*")
        || bytes.starts_with(b"export\n")
        || bytes.starts_with(b"export\r");

    if !is_import && !is_export {
        return None;
    }

    // Greedily consume all non-blank continuation lines.
    let mut ix = 0;
    loop {
        // Find end of current line.
        let eol = memchr(b'\n', &bytes[ix..])
            .map(|i| ix + i + 1)
            .unwrap_or(bytes.len());
        ix = eol;

        // Stop at EOF or blank line (line starting with \n or \r).
        if ix >= bytes.len() || bytes[ix] == b'\n' || bytes[ix] == b'\r' {
            break;
        }
    }
    Some(ix)
}

/// Check if `<` starts a **block-level** MDX JSX element.
/// A block JSX element must be the only significant content on the line
/// (possibly followed only by whitespace). If there's trailing text, it's
/// inline JSX inside a paragraph instead.
///
/// Returns byte offset past the element (including trailing newline) if matched.
pub(crate) fn scan_mdx_jsx_block(bytes: &[u8]) -> Option<usize> {
    if bytes.len() < 2 || bytes[0] != b'<' {
        return None;
    }

    let is_closing = bytes[1] == b'/';
    let name_start = if is_closing { 2 } else { 1 };

    if name_start >= bytes.len() {
        return None;
    }

    // Fragment: `<>` or `</>`
    if bytes[name_start] == b'>' {
        let after = name_start + 1;
        return if is_only_whitespace_to_eol(&bytes[after..]) {
            scan_to_line_end(bytes, after)
        } else {
            None // trailing content → inline
        };
    }

    // Tag name must start with an ASCII letter or `_`
    let first = bytes[name_start];
    if !first.is_ascii_alphabetic() && first != b'_' && first != b'$' && first < 0x80 {
        return None;
    }

    // Find the end of the first tag, then allow more tags or expressions
    // on the same line (e.g., `<a></a>`, `<a/>{1}`, `{1}<a/>`).
    let mut pos = scan_mdx_jsx_tag_end(bytes)?;

    // When the first tag is opening (not self-closing, not closing), anything
    // between it and its matching closer is element body. Accept more JSX
    // tags (which will be balanced in a later pass), but reject inline
    // `{expression}` and bare text — those are children that would be lost
    // if we claimed this line as a single flow element. Letting the line
    // fall through to paragraph parsing yields an `MdxJsxTextElement` with
    // the expression/text preserved as a child.
    let first_is_self_closing = pos >= 2 && bytes[pos - 2] == b'/' && bytes[pos - 1] == b'>';
    let first_is_opening = !is_closing && !first_is_self_closing;

    // Consume any subsequent tags or expressions on the same line.
    loop {
        // Skip whitespace
        while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
            pos += 1;
        }
        if pos >= bytes.len() || bytes[pos] == b'\n' || bytes[pos] == b'\r' {
            break; // EOL, valid flow
        }
        // Try another JSX tag
        if bytes[pos] == b'<' {
            if let Some(end) = scan_mdx_jsx_tag_end(&bytes[pos..]) {
                pos += end;
                continue;
            }
        }
        // Try an expression. When the first tag was opening (e.g. `<Foo>{x}</Foo>`),
        // the `{x}` is element body — refuse to claim the line as flow so it falls
        // through to paragraph parsing, where it becomes an `MdxJsxTextElement`
        // with the expression preserved as a child.
        if bytes[pos] == b'{' {
            if first_is_opening {
                return None;
            }
            if let Some((_, _, len)) = scan_mdx_inline_expression(&bytes[pos..]) {
                pos += len;
                continue;
            }
        }
        // Something else follows, not a flow element
        return None;
    }

    scan_to_line_end(bytes, pos)
}

/// Scan for an MDX expression block: `{...}` using lexer-based boundary detection.
/// Returns byte offset past the end (including trailing newline).
pub(crate) fn scan_mdx_expression_block(bytes: &[u8]) -> Option<usize> {
    let mut ix = scan_mdx_expression_end(bytes, false)?;

    // Block-level expression: only whitespace may follow on the line.
    if !is_only_whitespace_to_eol(&bytes[ix..]) {
        return None;
    }
    // Skip trailing whitespace and newline.
    while ix < bytes.len() && (bytes[ix] == b' ' || bytes[ix] == b'\t') {
        ix += 1;
    }
    if ix < bytes.len() && bytes[ix] == b'\r' {
        ix += 1;
    }
    if ix < bytes.len() && bytes[ix] == b'\n' {
        ix += 1;
    }
    Some(ix)
}

/// Scan an inline MDX expression: `{...}` using lexer-based boundary detection.
/// Returns (content_start, content_end, total_len) where content excludes the outer braces.
pub(crate) fn scan_mdx_inline_expression(bytes: &[u8]) -> Option<(usize, usize, usize)> {
    let total = scan_mdx_expression_end(bytes, true)?;
    Some((1, total - 1, total))
}

/// Scan an inline JSX tag from `<` to `>` or `/>`.
/// In MDX mode, ALL tags are JSX. Returns total byte length if matched.
pub(crate) fn scan_mdx_inline_jsx(bytes: &[u8]) -> Option<usize> {
    if bytes.len() < 2 || bytes[0] != b'<' {
        return None;
    }

    let is_closing = bytes[1] == b'/';
    let name_start = if is_closing { 2 } else { 1 };

    if name_start >= bytes.len() {
        return None;
    }

    // Fragment: `<>` or `</>`
    if bytes[name_start] == b'>' {
        return Some(name_start + 1);
    }

    // Must start with an ASCII letter, `_`, or `$`
    let first = bytes[name_start];
    if !first.is_ascii_alphabetic() && first != b'_' && first != b'$' && first < 0x80 {
        return None;
    }

    // Scan the tag name: letters, digits, `-`, `.` are valid name characters.
    let mut ix = name_start + 1;
    while ix < bytes.len() {
        match bytes[ix] {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'$' => ix += 1,
            // Namespace separator (e.g. `xml:space`): must be followed by a
            // valid name-start character.
            b':' => {
                ix += 1;
                if ix >= bytes.len() {
                    return None;
                }
                let ch = bytes[ix];
                if !ch.is_ascii_alphabetic() && ch != b'_' && ch != b'$' && ch < 0x80 {
                    return None;
                }
                ix += 1;
            }
            // Non-ASCII UTF-8 byte: allow (component names can use Unicode)
            0x80.. => ix += 1,
            _ => break,
        }
    }

    // After the tag name: must be whitespace, `>`, `/`, or `{`.
    // This rejects patterns like `<https://...>`.
    if ix < bytes.len() {
        match bytes[ix] {
            b'>' | b'/' | b' ' | b'\t' | b'\n' | b'\r' | b'{' => {}
            _ => return None,
        }
    }

    // Scan to closing `>` or `/>` handling balanced braces and strings
    let mut brace_depth: usize = 0;

    while ix < bytes.len() {
        match bytes[ix] {
            b'>' if brace_depth == 0 => return Some(ix + 1),
            b'/' if ix + 1 < bytes.len() && bytes[ix + 1] == b'>' && brace_depth == 0 => {
                return Some(ix + 2);
            }
            b'{' => {
                brace_depth += 1;
                ix += 1;
            }
            b'}' => {
                brace_depth = brace_depth.saturating_sub(1);
                ix += 1;
            }
            b'"' => {
                ix += 1;
                while ix < bytes.len() && bytes[ix] != b'"' {
                    if bytes[ix] == b'\\' {
                        ix += 1;
                    }
                    ix += 1;
                }
                if ix < bytes.len() {
                    ix += 1;
                }
            }
            b'\'' => {
                ix += 1;
                while ix < bytes.len() && bytes[ix] != b'\'' {
                    if bytes[ix] == b'\\' {
                        ix += 1;
                    }
                    ix += 1;
                }
                if ix < bytes.len() {
                    ix += 1;
                }
            }
            b'\n' | b'\r' => {
                // Multi-line JSX: keep scanning (attributes/values can span lines).
                ix += 1;
                if ix < bytes.len() && bytes[ix - 1] == b'\r' && bytes[ix] == b'\n' {
                    ix += 1;
                }
            }
            _ => ix += 1,
        }
    }
    None
}

impl<'a, 'b> FirstPass<'a, 'b> {
    pub(crate) fn parse_mdx_esm(&mut self, start_ix: usize, end_ix: usize) -> usize {
        let content = &self.text[start_ix..end_ix].trim_end();
        let cow_ix = self.allocs.allocate_cow((*content).into());
        self.tree.append(Item {
            start: start_ix,
            end: end_ix,
            body: ItemBody::MdxEsm(cow_ix),
        });
        end_ix
    }

    pub(crate) fn parse_mdx_jsx_flow(&mut self, start_ix: usize, end_ix: usize) -> usize {
        let raw = &self.text[start_ix..end_ix].trim_end();
        let jsx_data = parse_jsx_tag(raw);
        let jsx_ix = self.allocs.allocate_jsx_element(jsx_data);
        self.tree.append(Item {
            start: start_ix,
            end: end_ix,
            body: ItemBody::MdxJsxFlowElement(jsx_ix),
        });
        end_ix
    }

    pub(crate) fn parse_mdx_flow_expression(&mut self, start_ix: usize, end_ix: usize) -> usize {
        // Content is between the outer `{` and `}`.
        let raw = &self.text[start_ix..end_ix].trim_end();
        let inner = &raw[1..raw.len() - 1]; // strip `{` and `}`
        let cow_ix = self.allocs.allocate_cow(inner.into());
        self.tree.append(Item {
            start: start_ix,
            end: end_ix,
            body: ItemBody::MdxFlowExpression(cow_ix),
        });
        end_ix
    }
}

use crate::parse::{JsxAttr, JsxElementData};

/// Parse a raw JSX tag string into structured `JsxElementData`.
///
/// Handles opening, closing, self-closing tags, and fragments.
/// Attributes are extracted with zero-copy `CowStr::Borrowed` where possible.
pub(crate) fn parse_jsx_tag<'a>(raw: &'a str) -> JsxElementData<'a> {
    let s = raw.trim();

    // Closing tag: </Name>
    if let Some(rest) = s.strip_prefix("</") {
        let name = extract_tag_name(rest);
        return JsxElementData {
            name: name.into(),
            attrs: Vec::new(),
            raw: raw.into(),
            is_closing: true,
            is_self_closing: false,
        };
    }

    // Self-closing: ends with />
    let ends_self_close = s.ends_with("/>");

    // Extract name, skip leading '<'
    let name = extract_tag_name(&s[1..]);

    // Check for self-contained: <Name ...>...</Name> or <>...</>
    let is_self_contained = if !name.is_empty() {
        let close_tag = alloc::format!("</{name}>");
        s.contains(&*close_tag)
    } else {
        s.contains("</>")
    };

    let is_self_closing = ends_self_close || is_self_contained;

    // Parse attributes
    let attrs = parse_jsx_attrs(s);

    JsxElementData {
        name: name.into(),
        attrs,
        raw: raw.into(),
        is_closing: false,
        is_self_closing,
    }
}

fn extract_tag_name(s: &str) -> &str {
    let end = s
        .find(|c: char| c.is_whitespace() || c == '/' || c == '>' || c == '{')
        .unwrap_or(s.len());
    &s[..end]
}

/// Extract the opening tag portion (up to the first unbalanced `>`).
fn extract_opening_tag(text: &str) -> &str {
    let mut depth = 0i32;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut in_backtick = false;
    let mut prev = '\0';

    for (i, ch) in text.char_indices() {
        if in_single_quote {
            if ch == '\'' && prev != '\\' {
                in_single_quote = false;
            }
        } else if in_double_quote {
            if ch == '"' && prev != '\\' {
                in_double_quote = false;
            }
        } else if in_backtick {
            if ch == '`' && prev != '\\' {
                in_backtick = false;
            }
        } else {
            match ch {
                '\'' => in_single_quote = true,
                '"' => in_double_quote = true,
                '`' => in_backtick = true,
                '{' => depth += 1,
                '}' => depth -= 1,
                '>' if depth == 0 => return &text[..=i],
                _ => {}
            }
        }
        prev = ch;
    }
    text
}

fn parse_jsx_attrs<'a>(text: &'a str) -> Vec<JsxAttr<'a>> {
    let tag = extract_opening_tag(text);
    let bytes = tag.as_bytes();
    let len = bytes.len();

    let mut attrs = Vec::new();
    let mut i = 1; // skip '<'

    // Skip '/' for closing tags
    if i < len && bytes[i] == b'/' {
        i += 1;
    }

    // Skip whitespace
    while i < len && bytes[i].is_ascii_whitespace() {
        i += 1;
    }

    // Skip tag name
    while i < len
        && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'.' | b'-' | b':' | b'_'))
    {
        i += 1;
    }

    loop {
        // Skip whitespace
        while i < len && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= len {
            break;
        }
        if bytes[i] == b'>' || (bytes[i] == b'/' && i + 1 < len && bytes[i + 1] == b'>') {
            break;
        }

        // Spread expression: {...expr}
        if bytes[i] == b'{' {
            i += 1;
            let start = i;
            let mut depth = 1i32;
            while i < len && depth > 0 {
                match bytes[i] {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    b'\'' | b'"' | b'`' => {
                        let q = bytes[i];
                        i += 1;
                        while i < len && bytes[i] != q {
                            if bytes[i] == b'\\' {
                                i += 1;
                            }
                            i += 1;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            let value = tag[start..i.saturating_sub(1)].trim();
            attrs.push(JsxAttr::Spread(value.into()));
            continue;
        }

        // Attribute name
        let name_start = i;
        while i < len
            && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'-' | b':' | b'_'))
        {
            i += 1;
        }
        if i == name_start {
            i += 1;
            continue;
        }
        let name = &tag[name_start..i];

        // Skip whitespace
        while i < len && bytes[i].is_ascii_whitespace() {
            i += 1;
        }

        if i < len && bytes[i] == b'=' {
            i += 1;
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i >= len {
                attrs.push(JsxAttr::Boolean(name.into()));
                continue;
            }
            if bytes[i] == b'"' || bytes[i] == b'\'' {
                let q = bytes[i];
                i += 1;
                let val_start = i;
                while i < len && bytes[i] != q {
                    if bytes[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                let value = &tag[val_start..i];
                if i < len {
                    i += 1;
                }
                attrs.push(JsxAttr::Literal(name.into(), value.into()));
            } else if bytes[i] == b'{' {
                i += 1;
                let val_start = i;
                let mut depth = 1i32;
                while i < len && depth > 0 {
                    match bytes[i] {
                        b'{' => depth += 1,
                        b'}' => depth -= 1,
                        b'\'' | b'"' | b'`' => {
                            let q = bytes[i];
                            i += 1;
                            while i < len && bytes[i] != q {
                                if bytes[i] == b'\\' {
                                    i += 1;
                                }
                                i += 1;
                            }
                        }
                        _ => {}
                    }
                    i += 1;
                }
                let value = &tag[val_start..i.saturating_sub(1)];
                attrs.push(JsxAttr::Expression(name.into(), value.into()));
            } else {
                attrs.push(JsxAttr::Boolean(name.into()));
            }
        } else {
            attrs.push(JsxAttr::Boolean(name.into()));
        }
    }

    attrs
}
