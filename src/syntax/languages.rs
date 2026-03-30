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

// Only use named node types — no string literal patterns for keywords
// This avoids "Invalid node type" errors across grammar versions

const RUST_HIGHLIGHTS: &str = r##"
(line_comment) @comment
(block_comment) @comment
(string_literal) @string
(raw_string_literal) @string
(char_literal) @string
(boolean_literal) @constant
(integer_literal) @number
(float_literal) @number
(type_identifier) @type
(primitive_type) @type
(function_item name: (identifier) @function)
(call_expression function: (identifier) @function)
(macro_invocation macro: (identifier) @function)
(field_identifier) @property
(attribute_item) @attribute
(self) @keyword
(mutable_specifier) @keyword
(use_declaration) @keyword
(visibility_modifier) @keyword
"##;

const PYTHON_HIGHLIGHTS: &str = r##"
(comment) @comment
(string) @string
(integer) @number
(float) @number
(true) @constant
(false) @constant
(none) @constant
(function_definition name: (identifier) @function)
(call function: (identifier) @function)
(class_definition name: (identifier) @type)
(decorator) @attribute
(identifier) @variable
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
(function_declaration name: (identifier) @function)
(call_expression function: (identifier) @function)
(class_declaration name: (identifier) @type)
(property_identifier) @property
(shorthand_property_identifier) @property
(identifier) @variable
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
(function_declaration name: (identifier) @function)
(call_expression function: (identifier) @function)
(type_identifier) @type
(class_declaration name: (identifier) @type)
(property_identifier) @property
(identifier) @variable
"##;

const HTML_HIGHLIGHTS: &str = r##"
(comment) @comment
(tag_name) @keyword
(attribute_name) @property
(quoted_attribute_value) @string
(attribute_value) @string
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
(type_identifier) @type
(primitive_type) @type
(sized_type_specifier) @type
(function_declarator declarator: (identifier) @function)
(call_expression function: (identifier) @function)
(field_identifier) @property
(preproc_include) @keyword
(preproc_def) @keyword
(preproc_ifdef) @keyword
(preproc_if) @keyword
(preproc_else) @keyword
(preproc_endif) @keyword
"##;

const BASH_HIGHLIGHTS: &str = r##"
(comment) @comment
(string) @string
(raw_string) @string
(number) @number
(command_name) @function
(variable_name) @property
(variable_assignment name: (variable_name) @property)
(function_definition name: (word) @function)
"##;

const PHP_HIGHLIGHTS: &str = r##"
(comment) @comment
(string) @string
(integer) @number
(float) @number
(boolean) @constant
(null) @constant
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
"##;

const MD_HIGHLIGHTS: &str = r##"
(atx_heading) @keyword
(setext_heading) @keyword
(link_destination) @string
(link_text) @property
(code_span) @string
(fenced_code_block) @string
(block_quote) @comment
"##;
