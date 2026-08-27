//! Source-preserving formatting and `bitterasm.toml` configuration.

use crate::lexer;
use crate::token::TokenKind;

mod config;
mod generics;
mod indentation;
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
use signature::normalize_macro_signatures;
use wrapping::{split_inline_facets, wrap_code, wrap_comment};

/// Formats whitespace while retaining every source token and comment.
pub fn format_source(source: &str, config: &FormatConfig) -> Result<String, String> {
    let newline = match config.newline_style {
        NewlineStyle::Auto if source.contains("\r\n") => "\r\n",
        NewlineStyle::Windows => "\r\n",
        NewlineStyle::Auto | NewlineStyle::Unix => "\n",
    };

    let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
    let normalized = normalize_macro_signatures(&normalized, config);
    let normalized = collapse_short_generics(&normalized, config)?;
    let tokens = lexer::lex(&normalized).map_err(|error| format!("lex error: {error}"))?;
    let mut delimiters = Vec::new();
    let mut generic_depth: usize = 0;
    let mut facet_blocks = Vec::new();
    let mut blank_lines = 0usize;
    let mut result = Vec::new();
    let mut offset = 0usize;

    for line in normalized.split('\n') {
        let end = offset + line.len();
        let line_tokens: Vec<_> = tokens
            .iter()
            .filter(|token| token.span.start >= offset && token.span.start < end)
            .collect();
        let content = line.trim();

        if content.is_empty() {
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
            let depth_bias = facet_blocks.len() + usize::from(facet_indent);
            let line_depth = delimiters.len().saturating_sub(leading_closers)
                + generic_depth.saturating_sub(leading_generic_closers)
                + depth_bias;
            let indent = make_indent(line_depth, config);

            if content.starts_with('#') {
                result.extend(wrap_comment(content, &indent, config.comment_width));
            } else if let Some(lines) = split_inline_facets(
                content,
                &line_tokens,
                offset,
                line_depth,
                config,
            ) {
                result.extend(lines);
            } else {
                result.extend(wrap_code(
                    content,
                    &line_tokens,
                    offset,
                    &delimiters,
                    generic_depth,
                    depth_bias,
                    config,
                ));
            }

            let previous_depth = delimiters.len();
            update_delimiters(&mut delimiters, &line_tokens);
            update_generic_depth(&mut generic_depth, &line_tokens);
            if config.indent_facets
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
    fn indents_all_delimited_continuations() {
        let source = "const x = call(\nfirst,\nNested {\nvalue: [\n1,\n]\n}\n)\n";
        assert_eq!(
            format_source(source, &FormatConfig::default()).unwrap(),
            "const x = call(\n    first,\n    Nested {\n        value: [\n            1,\n        ]\n    }\n)\n"
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
