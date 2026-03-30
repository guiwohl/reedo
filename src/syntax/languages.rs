use tree_sitter::Language;

pub struct LangConfig {
    pub name: &'static str,
    pub language: Language,
    pub highlight_query: &'static str,
    pub extensions: &'static [&'static str],
}

pub fn all_languages() -> Vec<LangConfig> {
    vec![
        LangConfig {
            name: "rust",
            language: tree_sitter_rust::LANGUAGE.into(),
            highlight_query: RUST_HIGHLIGHTS,
            extensions: &["rs"],
        },
        LangConfig {
            name: "python",
            language: tree_sitter_python::LANGUAGE.into(),
            highlight_query: PYTHON_HIGHLIGHTS,
            extensions: &["py", "pyi"],
        },
        LangConfig {
            name: "javascript",
            language: tree_sitter_javascript::LANGUAGE.into(),
            highlight_query: JS_HIGHLIGHTS,
            extensions: &["js", "mjs", "cjs", "jsx"],
        },
        LangConfig {
            name: "typescript",
            language: tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            highlight_query: TS_HIGHLIGHTS,
            extensions: &["ts", "tsx"],
        },
        LangConfig {
            name: "html",
            language: tree_sitter_html::LANGUAGE.into(),
            highlight_query: HTML_HIGHLIGHTS,
            extensions: &["html", "htm"],
        },
        LangConfig {
            name: "css",
            language: tree_sitter_css::LANGUAGE.into(),
            highlight_query: CSS_HIGHLIGHTS,
            extensions: &["css", "scss"],
        },
        LangConfig {
            name: "c",
            language: tree_sitter_c::LANGUAGE.into(),
            highlight_query: C_HIGHLIGHTS,
            extensions: &["c", "h"],
        },
        LangConfig {
            name: "bash",
            language: tree_sitter_bash::LANGUAGE.into(),
            highlight_query: BASH_HIGHLIGHTS,
            extensions: &["sh", "bash", "zsh"],
        },
        LangConfig {
            name: "php",
            language: tree_sitter_php::LANGUAGE_PHP.into(),
            highlight_query: PHP_HIGHLIGHTS,
            extensions: &["php"],
        },
        LangConfig {
            name: "json",
            language: tree_sitter_json::LANGUAGE.into(),
            highlight_query: JSON_HIGHLIGHTS,
            extensions: &["json"],
        },
        LangConfig {
            name: "markdown",
            language: tree_sitter_md::LANGUAGE.into(),
            highlight_query: MD_HIGHLIGHTS,
            extensions: &["md", "markdown"],
        },
    ]
}

// highlight queries — these map tree-sitter node types to capture names
// capture names are then mapped to colors in the theme

const RUST_HIGHLIGHTS: &str = r##"
(line_comment) @comment
(block_comment) @comment
(string_literal) @string
(raw_string_literal) @string
(char_literal) @string
(boolean_literal) @constant
(integer_literal) @number
(float_literal) @number
"fn" @keyword
"let" @keyword
"mut" @keyword
"pub" @keyword
"mod" @keyword
"use" @keyword
"struct" @keyword
"enum" @keyword
"impl" @keyword
"trait" @keyword
"type" @keyword
"const" @keyword
"static" @keyword
"if" @keyword
"else" @keyword
"match" @keyword
"for" @keyword
"while" @keyword
"loop" @keyword
"return" @keyword
"break" @keyword
"continue" @keyword
"async" @keyword
"await" @keyword
"self" @keyword
"super" @keyword
"crate" @keyword
"as" @keyword
"in" @keyword
"where" @keyword
"ref" @keyword
"move" @keyword
"unsafe" @keyword
"extern" @keyword
"dyn" @keyword
(type_identifier) @type
(primitive_type) @type
(function_item name: (identifier) @function)
(call_expression function: (identifier) @function)
(macro_invocation macro: (identifier) @function.macro)
(field_identifier) @property
(attribute_item) @attribute
"#" @attribute
"!" @operator
"&" @operator
"*" @operator
"->" @operator
"=>" @operator
"::" @operator
"=" @operator
"==" @operator
"!=" @operator
"<" @operator
">" @operator
"<=" @operator
">=" @operator
"+" @operator
"-" @operator
"/" @operator
"%" @operator
"&&" @operator
"||" @operator
"##;

const PYTHON_HIGHLIGHTS: &str = r##"
(comment) @comment
(string) @string
(integer) @number
(float) @number
(true) @constant
(false) @constant
(none) @constant
"def" @keyword
"class" @keyword
"return" @keyword
"if" @keyword
"elif" @keyword
"else" @keyword
"for" @keyword
"while" @keyword
"import" @keyword
"from" @keyword
"as" @keyword
"with" @keyword
"try" @keyword
"except" @keyword
"finally" @keyword
"raise" @keyword
"pass" @keyword
"break" @keyword
"continue" @keyword
"and" @keyword
"or" @keyword
"not" @keyword
"in" @keyword
"is" @keyword
"lambda" @keyword
"yield" @keyword
"async" @keyword
"await" @keyword
"self" @variable.builtin
(function_definition name: (identifier) @function)
(call function: (identifier) @function)
(class_definition name: (identifier) @type)
(decorator) @attribute
"@" @attribute
"=" @operator
"==" @operator
"!=" @operator
"<" @operator
">" @operator
"+" @operator
"-" @operator
"*" @operator
"/" @operator
"%" @operator
"##;

const JS_HIGHLIGHTS: &str = r##"
(comment) @comment
(string) @string
(template_string) @string
(number) @number
(true) @constant
(false) @constant
(null) @constant
(undefined) @constant
"function" @keyword
"const" @keyword
"let" @keyword
"var" @keyword
"return" @keyword
"if" @keyword
"else" @keyword
"for" @keyword
"while" @keyword
"do" @keyword
"switch" @keyword
"case" @keyword
"default" @keyword
"break" @keyword
"continue" @keyword
"new" @keyword
"class" @keyword
"extends" @keyword
"import" @keyword
"export" @keyword
"from" @keyword
"async" @keyword
"await" @keyword
"try" @keyword
"catch" @keyword
"finally" @keyword
"throw" @keyword
"typeof" @keyword
"instanceof" @keyword
"this" @variable.builtin
(function_declaration name: (identifier) @function)
(call_expression function: (identifier) @function)
(arrow_function) @function
(class_declaration name: (identifier) @type)
(property_identifier) @property
"=" @operator
"==" @operator
"===" @operator
"!=" @operator
"!==" @operator
"=>" @operator
"+" @operator
"-" @operator
"*" @operator
"/" @operator
"%" @operator
"&&" @operator
"||" @operator
"!" @operator
"##;

const TS_HIGHLIGHTS: &str = r##"
(comment) @comment
(string) @string
(template_string) @string
(number) @number
(true) @constant
(false) @constant
(null) @constant
(undefined) @constant
"function" @keyword
"const" @keyword
"let" @keyword
"var" @keyword
"return" @keyword
"if" @keyword
"else" @keyword
"for" @keyword
"while" @keyword
"do" @keyword
"switch" @keyword
"case" @keyword
"default" @keyword
"break" @keyword
"continue" @keyword
"new" @keyword
"class" @keyword
"extends" @keyword
"import" @keyword
"export" @keyword
"from" @keyword
"async" @keyword
"await" @keyword
"try" @keyword
"catch" @keyword
"finally" @keyword
"throw" @keyword
"typeof" @keyword
"instanceof" @keyword
"interface" @keyword
"type" @keyword
"enum" @keyword
"implements" @keyword
"this" @variable.builtin
(function_declaration name: (identifier) @function)
(call_expression function: (identifier) @function)
(type_identifier) @type
(class_declaration name: (identifier) @type)
(property_identifier) @property
"=" @operator
"==" @operator
"===" @operator
"!=" @operator
"!==" @operator
"=>" @operator
":" @operator
"?" @operator
"+" @operator
"-" @operator
"*" @operator
"/" @operator
"##;

const HTML_HIGHLIGHTS: &str = r##"
(comment) @comment
(tag_name) @keyword
(attribute_name) @property
(quoted_attribute_value) @string
(attribute_value) @string
(doctype) @keyword
"<" @operator
">" @operator
"</" @operator
"/>" @operator
"=" @operator
"##;

const CSS_HIGHLIGHTS: &str = r##"
(comment) @comment
(tag_name) @keyword
(class_name) @type
(id_name) @constant
(property_name) @property
(string_value) @string
(color_value) @number
(integer_value) @number
(float_value) @number
(plain_value) @string
(important) @keyword
"@" @keyword
":" @operator
";" @operator
"##;

const C_HIGHLIGHTS: &str = r##"
(comment) @comment
(string_literal) @string
(system_lib_string) @string
(char_literal) @string
(number_literal) @number
(true) @constant
(false) @constant
(null) @constant
"if" @keyword
"else" @keyword
"for" @keyword
"while" @keyword
"do" @keyword
"switch" @keyword
"case" @keyword
"default" @keyword
"break" @keyword
"continue" @keyword
"return" @keyword
"struct" @keyword
"typedef" @keyword
"enum" @keyword
"union" @keyword
"sizeof" @keyword
"static" @keyword
"extern" @keyword
"const" @keyword
"void" @keyword
"#include" @keyword
"#define" @keyword
"#ifdef" @keyword
"#ifndef" @keyword
"#endif" @keyword
"#if" @keyword
"#else" @keyword
(type_identifier) @type
(primitive_type) @type
(function_declarator declarator: (identifier) @function)
(call_expression function: (identifier) @function)
(field_identifier) @property
(preproc_directive) @keyword
"=" @operator
"==" @operator
"!=" @operator
"<" @operator
">" @operator
"+" @operator
"-" @operator
"*" @operator
"/" @operator
"%" @operator
"&&" @operator
"||" @operator
"!" @operator
"->" @operator
"##;

const BASH_HIGHLIGHTS: &str = r##"
(comment) @comment
(string) @string
(raw_string) @string
(number) @number
(command_name) @function
(variable_name) @property
"if" @keyword
"then" @keyword
"else" @keyword
"elif" @keyword
"fi" @keyword
"for" @keyword
"while" @keyword
"do" @keyword
"done" @keyword
"case" @keyword
"esac" @keyword
"in" @keyword
"function" @keyword
"return" @keyword
"local" @keyword
"export" @keyword
"readonly" @keyword
"unset" @keyword
"$" @operator
"|" @operator
">" @operator
"<" @operator
">>" @operator
"&&" @operator
"||" @operator
"=" @operator
"##;

const PHP_HIGHLIGHTS: &str = r##"
(comment) @comment
(string) @string
(integer) @number
(float) @number
(boolean) @constant
(null) @constant
"function" @keyword
"class" @keyword
"public" @keyword
"private" @keyword
"protected" @keyword
"static" @keyword
"return" @keyword
"if" @keyword
"else" @keyword
"elseif" @keyword
"for" @keyword
"foreach" @keyword
"while" @keyword
"do" @keyword
"switch" @keyword
"case" @keyword
"default" @keyword
"break" @keyword
"continue" @keyword
"new" @keyword
"echo" @keyword
"try" @keyword
"catch" @keyword
"finally" @keyword
"throw" @keyword
"use" @keyword
"namespace" @keyword
"extends" @keyword
"implements" @keyword
"interface" @keyword
"abstract" @keyword
"final" @keyword
"const" @keyword
"$" @operator
"->" @operator
"=>" @operator
"::" @operator
"=" @operator
"==" @operator
"===" @operator
"!=" @operator
"!==" @operator
(name) @function
(class_declaration name: (name) @type)
(named_type (name) @type)
(variable_name) @property
"##;

const JSON_HIGHLIGHTS: &str = r##"
(string) @string
(number) @number
(true) @constant
(false) @constant
(null) @constant
(pair key: (string) @property)
":" @operator
"," @operator
"##;

const MD_HIGHLIGHTS: &str = r##"
(atx_heading) @keyword
(setext_heading) @keyword
(link_destination) @string
(link_text) @property
(emphasis) @keyword
(strong_emphasis) @keyword
(code_span) @string
(fenced_code_block) @string
(block_quote) @comment
(list_marker_plus) @operator
(list_marker_minus) @operator
(list_marker_star) @operator
(list_marker_dot) @operator
"##;
