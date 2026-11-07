//! Source-preserving formatting and `bitterasm.toml` configuration.

use crate::lexer;
use crate::token::TokenKind;

mod config;
mod generics;
mod indentation;
mod labels;
mod patterns;
mod signature;
mod wrapping;

pub use config::{discover_config, load_config, FormatConfig, NewlineStyle};
#[cfg(test)]
use config::parse_config;
use generics::collapse_short_generics;
use indentation::{
    contains_facet, is_facet, leading_closers, leading_generic_closers, make_indent,
    update_delimiters, update_generic_depth,
};
use labels::label_body_lines;
use patterns::split_facet_body;
use signature::normalize_macro_signatures;
use wrapping::{split_inline_facets, wrap_code, wrap_comment, wrap_doc_comment, Surroundings};

/// Formats whitespace while retaining every source token and comment.
/// The most formatting passes `format_source` runs while the output keeps
/// changing. One pass wraps one list per long line, so a line with several
/// long lists needs a pass for each.
const MAX_PASSES: usize = 8;

/// Formats `source`. The result is stable: formatting it again changes
/// nothing, which is what `bitterasm format --check` relies on.
pub fn format_source(source: &str, config: &FormatConfig) -> Result<String, String> {
    let mut formatted = format_pass(source, config)?;
    for _ in 1..MAX_PASSES {
        let again = format_pass(&formatted, config)?;
        if again == formatted {
            break;
        }
        formatted = again;
    }
    Ok(formatted)
}

fn format_pass(source: &str, config: &FormatConfig) -> Result<String, String> {
    let newline = match config.newline_style {
        NewlineStyle::Auto if source.contains("\r\n") => "\r\n",
        NewlineStyle::Windows => "\r\n",
        NewlineStyle::Auto | NewlineStyle::Unix => "\n",
    };

    let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
    let normalized = normalize_macro_signatures(&normalized, config);
    let normalized = collapse_short_generics(&normalized, config)?;
    let tokens = lexer::lex(&normalized).map_err(|error| format!("lex error: {error}"))?;
    let label_body = if config.indent_label_bodies {
        label_body_lines(&normalized, &tokens)
    } else {
        Vec::new()
    };
    let mut delimiters = Vec::new();
    // The line each open delimiter was opened on, in step with `delimiters`.
    let mut opened_on: Vec<usize> = Vec::new();
    let mut generic_depth: usize = 0;
    let mut facet_blocks = Vec::new();
    let mut blank_lines = 0usize;
    let mut in_doc_fence = false;
    // Whether this line continues an `@` statement that broke after a comma
    // outside any brackets, like `@next a = 1,` / `b = 2`.
    let mut continues = false;
    let mut result = Vec::new();
    let mut offset = 0usize;

    for (line_index, line) in normalized.split('\n').enumerate() {
        let end = offset + line.len();
        let line_tokens: Vec<_> = tokens
            .iter()
            .filter(|token| token.span.start >= offset && token.span.start < end)
            .collect();
        let content = line.trim();

        if content.is_empty() {
            continues = false;
            if !result.is_empty() && blank_lines < config.max_blank_lines {
                result.push(String::new());
            }
            blank_lines += 1;
        } else {
            blank_lines = 0;

            // Separate adjacent top-level block declarations. Comments stay
            // attached to the declaration they immediately precede because
            // insertion only happens directly after a closing brace.
            if delimiters.is_empty()
                && result.last().is_some_and(|line| line.trim() == "}")
                && is_top_level_declaration(content)
            {
                result.push(String::new());
            }
            let leading_closers = leading_closers(&line_tokens);
            let leading_generic_closers = leading_generic_closers(&line_tokens);
            let is_facet = is_facet(&line_tokens);
            let facet_indent = config.indent_facets && is_facet;
            let label_indent = label_body.get(line_index).copied().unwrap_or(false);
            let depth_bias = facet_blocks.len()
                + usize::from(facet_indent)
                + usize::from(label_indent)
                + usize::from(continues);
            // Brackets opened together on one line, like `Node(BinOp(`,
            // indent what's inside them once, not once each.
            let still_open = &opened_on[..opened_on.len().saturating_sub(leading_closers)];
            let shared = shared_openers(still_open);
            let line_depth = delimiters.len().saturating_sub(leading_closers + shared)
                + generic_depth.saturating_sub(leading_generic_closers)
                + depth_bias;
            let indent = make_indent(line_depth, config);
            let mut body_split = false;
            let split_closers = split_leading_closers(content, &line_tokens, offset, &opened_on);

            if lexer::doc_comment(content).is_some() {
                result.extend(wrap_doc_comment(content, &indent, config.comment_width, &mut in_doc_fence));
            } else if content.starts_with('#') {
                result.extend(wrap_comment(content, &indent, config.comment_width));
            } else if let Some(lines) = split_inline_facets(
                content,
                &line_tokens,
                offset,
                line_depth,
                config,
            ) {
                result.extend(lines);
            } else if let Some(first) = split_closers {
                // `) }` closing brackets opened on different lines: each
                // goes on its own line, where it would line up alone. The
                // rest keeps this line's indentation.
                let still_open = &opened_on[..opened_on.len() - first.count];
                let depth = delimiters.len() - first.count - shared_openers(still_open)
                    + generic_depth.saturating_sub(leading_generic_closers)
                    + depth_bias;
                result.push(format!("{}{}", make_indent(depth, config), content[..first.end].trim()));
                result.push(format!("{indent}{}", content[first.end..].trim()));
            } else if let Some(lines) = split_facet_body(
                content,
                &line_tokens,
                offset,
                line_depth,
                line_depth.saturating_sub(usize::from(facet_indent)),
                config,
            ) {
                body_split = true;
                result.extend(lines);
            } else {
                result.extend(wrap_code(
                    content,
                    &line_tokens,
                    offset,
                    &Surroundings { outer: &delimiters, shared, generic_depth, depth_bias },
                    config,
                ));
            }

            let previous_depth = delimiters.len();
            update_delimiters(&mut delimiters, &line_tokens);
            if !line_tokens.is_empty() {
                continues = (continues || line_tokens[0].kind == TokenKind::At)
                    && line_tokens.last().is_some_and(|token| token.kind == TokenKind::Comma)
                    && delimiters.len() == previous_depth;
            }
            update_opened_on(&mut opened_on, &line_tokens, line_index);
            update_generic_depth(&mut generic_depth, &line_tokens);
            // A body moved off its facet line is indented as the
            // declaration's, not the facet's.
            if config.indent_facets
                && !body_split
                && (facet_indent || contains_facet(&line_tokens))
                && line_tokens.iter().any(|token| token.kind == TokenKind::LBrace)
                && delimiters.len() > previous_depth
            {
                facet_blocks.push(delimiters.len());
            }
            facet_blocks.retain(|depth| *depth <= delimiters.len());
        }
        offset = end + 1;
    }

    while result.last().is_some_and(String::is_empty) {
        result.pop();
    }
    Ok(result.join(newline) + newline)
}

/// The closers a line starts with that were all opened on one line, when
/// the line goes on to close brackets from another line too.
struct FirstClosers {
    count: usize,
    end: usize,
}

fn split_leading_closers(
    content: &str,
    tokens: &[&crate::token::Token],
    line_offset: usize,
    opened_on: &[usize],
) -> Option<FirstClosers> {
    let closers = leading_closers(tokens).min(opened_on.len());
    let innermost = *opened_on.last()?;
    let count = opened_on[opened_on.len() - closers..]
        .iter()
        .rev()
        .take_while(|line| **line == innermost)
        .count();
    if count == 0 || count == closers {
        return None;
    }
    let source_start = tokens.first().map_or(line_offset, |token| token.span.start);
    let end = tokens[count - 1].span.end - source_start;
    (end < content.len()).then_some(FirstClosers { count, end })
}

/// How many of these open delimiters were opened on the same line as the
/// one before them.
fn shared_openers(opened_on: &[usize]) -> usize {
    opened_on.windows(2).filter(|pair| pair[0] == pair[1]).count()
}

fn update_opened_on(opened_on: &mut Vec<usize>, tokens: &[&crate::token::Token], line_index: usize) {
    for token in tokens {
        match token.kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => opened_on.push(line_index),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                opened_on.pop();
            }
            _ => {}
        }
    }
}

fn is_top_level_declaration(content: &str) -> bool {
    let content = content.strip_prefix("pub ").unwrap_or(content);
    ["macro ", "struct ", "enum ", "type "]
        .iter()
        .any(|prefix| content.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_blocks_without_losing_comments() {
        let source = "macro x()\n {\n# keep me\n@emit 1   \n}\n\n\n";
        assert_eq!(
            format_source(source, &FormatConfig::default()).unwrap(),
            "macro x()\n{\n    # keep me\n    @emit 1\n}\n"
        );
    }

    #[test]
    fn does_not_wrap_meta_arguments_at_a_statement_level_comma() {
        let config = FormatConfig { max_width: 60, ..Default::default() };
        let source = "macro checked(precision: int) -> int {\n@assert precision >= 0 && precision <= 60, \"precision must be between zero and sixty\"\n@return precision\n}\n";
        let formatted = format_source(source, &config).unwrap();
        assert!(formatted.contains("@assert precision >= 0 && precision <= 60, \"precision"));
        let tokens = crate::lexer::lex(&formatted).unwrap();
        crate::parser::parse(tokens).expect("formatted output should remain parseable");
    }

    #[test]
    fn never_wraps_inside_a_syntax_pattern() {
        let config = FormatConfig { max_width: 60, ..Default::default() };
        let source = "syntax mov(a, b) = { mov $a$, $b$, $a$, $b$, $a$, $b$, $a$, $b$, $a$ }\n";
        let formatted = format_source(source, &config).unwrap();
        assert!(formatted.contains("= { mov $a$, $b$, $a$, $b$, $a$, $b$, $a$, $b$, $a$ }"));
        let source = "macro m(a: int, b: int)\n    | syntax { [$a$+$b$] } { @return Pair(a, b, a, b, a, b, a) }\n";
        let formatted = format_source(source, &config).unwrap();
        assert!(formatted.contains("| syntax { [$a$+$b$] }\n"), "{formatted}");
    }

    #[test]
    fn moves_a_long_syntax_body_onto_its_own_lines() {
        let config = FormatConfig { max_width: 50, ..Default::default() };
        let source = "macro m(a: int, b: int)\n    | syntax { m $a$, $b$ } { @return Pair(a, b, a, b) }\n";
        assert_eq!(
            format_source(source, &config).unwrap(),
            "macro m(a: int, b: int)\n    | syntax { m $a$, $b$ }\n{\n    @return Pair(a, b, a, b)\n}\n"
        );
    }

    #[test]
    fn keeps_a_short_syntax_body_on_the_facet_line() {
        let source = "macro m(a: int)\n    | syntax { m $a$ } { @emit a }\n";
        assert_eq!(format_source(source, &FormatConfig::default()).unwrap(), source);
    }

    #[test]
    fn a_multiline_syntax_body_opens_on_its_own_line() {
        let source = "macro m(a: int)\n    | syntax { m $a$ } {\n        @emit a\n        @emit a\n    }\n";
        assert_eq!(
            format_source(source, &FormatConfig::default()).unwrap(),
            "macro m(a: int)\n    | syntax { m $a$ }\n{\n    @emit a\n    @emit a\n}\n"
        );
    }

    #[test]
    fn wraps_the_list_with_the_commas_not_an_earlier_bracket_at_its_depth() {
        let config = FormatConfig { max_width: 40, ..Default::default() };
        let source = "macro z() -> int {\n@return f(x)[0] + g(aaaaaaaa, bbbbbbbbbb, cccccccccc)\n}\n";
        assert_eq!(
            format_source(source, &config).unwrap(),
            "macro z() -> int {\n    @return f(x)[0] + g(\n        aaaaaaaa,\n        bbbbbbbbbb,\n        cccccccccc\n    )\n}\n"
        );
    }

    #[test]
    fn a_label_followed_by_a_blank_line_has_no_body() {
        let source = "start:\nnop\nstart_end:\n\nheader\n";
        assert_eq!(
            format_source(source, &FormatConfig::default()).unwrap(),
            "start:\n    nop\nstart_end:\n\nheader\n"
        );
    }

    #[test]
    fn brackets_opened_on_one_line_indent_once() {
        let source = "macro m() {\n@return Node(Pair(\na = 1,\nb = 2\n))\n}\n";
        assert_eq!(
            format_source(source, &FormatConfig::default()).unwrap(),
            "macro m() {\n    @return Node(Pair(\n        a = 1,\n        b = 2\n    ))\n}\n"
        );
    }

    #[test]
    fn closers_from_different_lines_go_on_their_own_lines() {
        let source = "macro m()\n    | emits Bytes<...>\n{\n@emit f(\n1,\n2\n) }\n";
        assert_eq!(
            format_source(source, &FormatConfig::default()).unwrap(),
            "macro m()\n    | emits Bytes<...>\n{\n    @emit f(\n        1,\n        2\n    )\n}\n"
        );
    }

    #[test]
    fn a_trailing_comment_does_not_make_a_line_wrap() {
        let config = FormatConfig { max_width: 30, ..Default::default() };
        let source = "field add(a, b), 4    # a comment past the width\n";
        assert_eq!(format_source(source, &config).unwrap(), source);
    }

    #[test]
    fn moves_an_emits_body_off_its_facet_line() {
        let source = "macro m()\n    | emits Bytes<...> {\n    @emit 1\n}\n";
        assert_eq!(
            format_source(source, &FormatConfig::default()).unwrap(),
            "macro m()\n    | emits Bytes<...>\n{\n    @emit 1\n}\n"
        );
    }

    #[test]
    fn indents_the_continuation_of_a_meta_statement() {
        let source = "macro m() {\n@next a = 1,\nb = 2,\nc = 3\n@emit a\n}\n";
        assert_eq!(
            format_source(source, &FormatConfig::default()).unwrap(),
            "macro m() {\n    @next a = 1,\n        b = 2,\n        c = 3\n    @emit a\n}\n"
        );
    }

    #[test]
    fn inserts_one_blank_line_between_top_level_block_declarations() {
        let source = "macro first() {\n@emit 1\n}\nmacro second() {\n@emit 2\n}\n";
        assert_eq!(
            format_source(source, &FormatConfig::default()).unwrap(),
            "macro first() {\n    @emit 1\n}\n\nmacro second() {\n    @emit 2\n}\n"
        );
    }

    #[test]
    fn formatted_inline_if_remains_parseable() {
        let source = "macro choose(x: int) -> int { @if x > 0 { @return 1 }\n@return 0 }\n";
        let formatted = format_source(source, &FormatConfig::default()).unwrap();
        let tokens = crate::lexer::lex(&formatted).unwrap();
        crate::parser::parse(tokens).expect("formatted inline blocks should remain parseable");
    }

    #[test]
    fn applies_config() {
        let config = FormatConfig { indent_width: 2, max_blank_lines: 0, ..Default::default() };
        assert_eq!(format_source("struct X\n{\nvalue: Int\n}\n", &config).unwrap(),
                   "struct X\n{\n  value: Int\n}\n");
    }

    #[test]
    fn parses_rustfmt_style_config() {
        let config = parse_config("indent_width = 2\nhard_tabs = true\nindent_facets = false\nfacets_on_new_line = false\npub_on_declaration = false\nreturn_type_on_declaration = false\nmax_width = 90\ncomment_width = 70\nnewline_style = \"Unix\"\n").unwrap();
        assert_eq!(config.indent_width, 2);
        assert!(config.hard_tabs);
        assert!(!config.indent_facets);
        assert!(!config.facets_on_new_line);
        assert!(!config.pub_on_declaration);
        assert!(!config.return_type_on_declaration);
        assert_eq!(config.max_width, 90);
        assert_eq!(config.comment_width, 70);
        assert_eq!(config.newline_style, NewlineStyle::Unix);
    }

    #[test]
    fn formatter_ignores_lint_configuration_section() {
        let config = parse_config(
            "indent_width = 2\n\n[lints]\nunused = \"deny\"\nunreachable_code = \"allow\"\n",
        )
        .unwrap();
        assert_eq!(config.indent_width, 2);
    }

    #[test]
    fn indents_all_delimited_continuations() {
        let source = "const x = call(\nfirst,\nNested {\nvalue: [\n1,\n]\n}\n)\n";
        assert_eq!(
            format_source(source, &FormatConfig::default()).unwrap(),
            "const x = call(\n    first,\n    Nested {\n        value: [\n            1,\n        ]\n    }\n)\n"
        );
    }

    #[test]
    fn keeps_doc_comment_markdown_intact() {
        let config = FormatConfig { comment_width: 20, ..Default::default() };
        let source = concat!(
            "#! one two three four five\n",
            "#! ```\n",
            "#! mov  rax,   rbx   ; a long line of code\n",
            "#! ```\n",
            "##   - one two three four five\n",
            "## | a   | b |\n",
            "##\n",
            "const x = 1\n",
        );
        assert_eq!(
            format_source(source, &config).unwrap(),
            concat!(
                "#! one two three\n",
                "#! four five\n",
                "#! ```\n",
                "#! mov  rax,   rbx   ; a long line of code\n",
                "#! ```\n",
                "##   - one two three\n",
                "##   four five\n",
                "## | a   | b |\n",
                "##\n",
                "const x = 1\n",
            )
        );
    }

    #[test]
    fn wraps_comments_and_safe_code_lists_separately() {
        let config = FormatConfig { max_width: 26, comment_width: 20, ..Default::default() };
        let source = "# one two three four five six\nconst x = call(first, second, third, fourth)\n";
        assert_eq!(
            format_source(source, &config).unwrap(),
            "# one two three four\n# five six\nconst x = call(\n    first,\n    second,\n    third,\n    fourth\n)\n"
        );
    }

    #[test]
    fn lays_out_a_long_emit_call_one_argument_per_line() {
        let config = FormatConfig { max_width: 70, ..Default::default() };
        let source = "    @emit IType(imm = Imm12(offset & 0xFFF), rs1 = rs1, funct3 = Funct3(0b010),\n        rd = rd,\n        opcode = Opcode(0b0000011))\n";
        assert_eq!(
            format_source(source, &config).unwrap(),
            "@emit IType(\n    imm = Imm12(offset & 0xFFF),\n    rs1 = rs1,\n    funct3 = Funct3(0b010),\n    rd = rd,\n    opcode = Opcode(0b0000011)\n)\n"
        );
    }

    #[test]
    fn indents_facets_relative_to_their_declaration() {
        let source = "macro x()\n| invariant enabled\n| before {\n@emit 1\n}\n{\n@emit 2\n}\n";
        assert_eq!(
            format_source(source, &FormatConfig::default()).unwrap(),
            "macro x()\n    | invariant enabled\n    | before {\n        @emit 1\n    }\n{\n    @emit 2\n}\n"
        );
    }

    #[test]
    fn facet_indentation_can_be_disabled() {
        let config = FormatConfig { indent_facets: false, ..Default::default() };
        assert_eq!(
            format_source("macro x()\n    | invariant enabled\n{}\n", &config).unwrap(),
            "macro x()\n| invariant enabled\n{}\n"
        );
    }

    #[test]
    fn indents_the_lines_under_a_label() {
        let source = "section .text\npub _start:\nmov eax, 1\n\n.write:\nsyscall # write\n";
        assert_eq!(
            format_source(source, &FormatConfig::default()).unwrap(),
            "section .text\npub _start:\n    mov eax, 1\n\n.write:\n    syscall # write\n"
        );
    }

    #[test]
    fn a_label_body_keeps_its_consts_and_ends_at_a_section() {
        let source = "msg:\nconst text = \"hi\"\ndb text\nsection .text\nret\n";
        assert_eq!(
            format_source(source, &FormatConfig::default()).unwrap(),
            "msg:\n    const text = \"hi\"\n    db text\nsection .text\nret\n"
        );
    }

    #[test]
    fn a_label_body_ends_at_a_declaration() {
        let source = "start:\nnoop\nmacro noop() {\n@emit 1\n}\n";
        assert_eq!(
            format_source(source, &FormatConfig::default()).unwrap(),
            "start:\n    noop\nmacro noop() {\n    @emit 1\n}\n"
        );
    }

    #[test]
    fn a_comment_above_a_label_stays_with_that_label() {
        let source = "start:\nnoop\n\n# the loop\nloop:\n# body\nnoop\n";
        assert_eq!(
            format_source(source, &FormatConfig::default()).unwrap(),
            "start:\n    noop\n\n# the loop\nloop:\n    # body\n    noop\n"
        );
    }

    #[test]
    fn a_multiline_invocation_under_a_label_is_indented_as_a_whole() {
        let source = "start:\nemit_pair(\n1,\n2\n)\n";
        assert_eq!(
            format_source(source, &FormatConfig::default()).unwrap(),
            "start:\n    emit_pair(\n        1,\n        2\n    )\n"
        );
    }

    #[test]
    fn labels_inside_a_macro_body_are_left_alone() {
        let source = "macro x() {\ninner:\n@emit 1\n}\n";
        assert_eq!(
            format_source(source, &FormatConfig::default()).unwrap(),
            "macro x() {\n    inner:\n    @emit 1\n}\n"
        );
    }

    #[test]
    fn label_body_indentation_can_be_disabled() {
        let config = FormatConfig { indent_label_bodies: false, ..Default::default() };
        let source = "start:\n    noop\n";
        assert_eq!(format_source(source, &config).unwrap(), "start:\nnoop\n");
    }

    #[test]
    fn generic_argument_commas_are_not_wrapped_as_parameters() {
        let config = FormatConfig { max_width: 45, ..Default::default() };
        let source = "pub macro updated(arr: Array<T, ...>, index: int, value: T)\n";
        assert_eq!(
            format_source(source, &config).unwrap(),
            "pub macro updated(\n    arr: Array<T, ...>,\n    index: int,\n    value: T\n)\n"
        );
    }

    #[test]
    fn indents_multiline_type_facets() {
        let source = "pub type uint8_t = bits<8>\n| invariant value >= 0\n";
        assert_eq!(
            format_source(source, &FormatConfig::default()).unwrap(),
            "pub type uint8_t = bits<8>\n    | invariant value >= 0\n"
        );
    }

    #[test]
    fn moves_inline_type_facets_onto_new_lines() {
        let source = "pub type uint8_t = bits<8> | invariant value >= 0\n";
        assert_eq!(
            format_source(source, &FormatConfig::default()).unwrap(),
            "pub type uint8_t = bits<8>\n    | invariant value >= 0\n"
        );
    }

    #[test]
    fn moving_facets_onto_new_lines_can_be_disabled() {
        let config = FormatConfig { facets_on_new_line: false, ..Default::default() };
        let source = "type Byte = bits<8> | invariant value >= 0\n";
        assert_eq!(format_source(source, &config).unwrap(), source);
    }

    #[test]
    fn moves_pub_and_return_type_to_the_macro_declaration() {
        let source = "macro encode(value: int)\n    | syntax \"encode $value$\"\n    | pub\n    | -> bits<8>\n{\n}\n";
        assert_eq!(
            format_source(source, &FormatConfig::default()).unwrap(),
            "pub macro encode(value: int) -> bits<8>\n    | syntax \"encode $value$\"\n{\n}\n"
        );
    }

    #[test]
    fn pub_and_return_type_signature_rules_are_independent() {
        let config = FormatConfig {
            pub_on_declaration: false,
            return_type_on_declaration: true,
            ..Default::default()
        };
        let source = "macro encode()\n| pub\n| -> int\n{\n}\n";
        assert_eq!(
            format_source(source, &config).unwrap(),
            "macro encode() -> int\n    | pub\n{\n}\n"
        );

        let config = FormatConfig {
            pub_on_declaration: true,
            return_type_on_declaration: false,
            ..Default::default()
        };
        assert_eq!(
            format_source(source, &config).unwrap(),
            "pub macro encode()\n    | -> int\n{\n}\n"
        );
    }

    #[test]
    fn keeps_a_short_signature_that_opens_its_body_on_one_line() {
        let source = "macro add(x: int, y: int) -> int {\n    @return x + y\n}\n";
        assert_eq!(format_source(source, &FormatConfig::default()).unwrap(), source);
    }

    #[test]
    fn still_lays_out_a_list_left_open_one_item_per_line() {
        let source = "const x = call(first, second,\nthird)\n";
        assert_eq!(
            format_source(source, &FormatConfig::default()).unwrap(),
            "const x = call(\n    first,\n    second,\n    third\n)\n"
        );
    }

    #[test]
    fn puts_the_closer_of_an_outer_list_on_its_own_line() {
        let source = "macro apply(x: int, f: macro(\nint\n) -> int) -> int {\n@return f(x)\n}\n";
        assert_eq!(
            format_source(source, &FormatConfig::default()).unwrap(),
            "macro apply(\n    x: int,\n    f: macro(\n        int\n    ) -> int\n) -> int {\n    @return f(x)\n}\n"
        );
    }

    #[test]
    fn wraps_every_long_list_on_a_line_in_one_run() {
        let config = FormatConfig { max_width: 40, ..Default::default() };
        let source = "const x = f(aaaa, bbbb, cccc) | f(dddd, eeee, ffff) | f(gggg, hhhh)\n";
        let once = format_source(source, &config).unwrap();
        assert_eq!(format_source(&once, &config).unwrap(), once);
        assert!(once.lines().all(|line| line.len() <= 40), "{once}");
    }

    #[test]
    fn rejoins_short_multiline_generic_arguments() {
        let source = "    @return Range<start,\n        stop,\n        step>{\n    }\n";
        assert_eq!(
            format_source(source, &FormatConfig::default()).unwrap(),
            "@return Range<start, stop, step>{\n}\n"
        );
    }

    #[test]
    fn collapsing_multiline_generics_can_be_disabled() {
        let config = FormatConfig {
            collapse_short_multiline_generics: false,
            ..Default::default()
        };
        let source = "type Pair = Tuple<\n    int,\n    int\n>\n";
        assert_eq!(
            format_source(source, &config).unwrap(),
            "type Pair = Tuple<\n    int,\n    int\n>\n"
        );
    }
}
