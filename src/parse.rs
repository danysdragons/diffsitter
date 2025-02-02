//! Utilities for reading and parsing files with the diffsitter parser

include!(concat!(env!("OUT_DIR"), "/generated_grammar.rs"));

use anyhow::{format_err, Result};
use log::{debug, info};
use logging_timer::time;
use std::collections::HashMap;
use std::{fs, path::Path};
use tree_sitter::{Parser, Tree};

/// A mapping of file extensions to their associated languages
///
/// The languages correspond to grammars from `tree-sitter`
static FILE_EXTS: phf::Map<&'static str, &'static str> = phf_map! {
    "hs" => "haskell",
    "rs" => "rust",
    "go" => "go",
    "c" => "c",
    "cc" => "cpp",
    "cpp" => "cpp",
    "cs" => "c_sharp",
    "java" => "java",
    "py" => "python",
    "css" => "css",
    "sh" => "bash",
    "bash" => "bash",
    "jl" => "julia",
    "ml" => "ocaml",
    "rb" => "ruby",
    "scala" => "scala",
    "sc" => "scala",
    "swift" => "swift",
    "php" => "php",
    "json" => "json",
    "hcl" => "hcl",
};

/// Generate a [tree sitter language](Language) from a language string
///
/// This will return an error if an unknown string is provided
fn generate_language(lang: &str) -> Result<Language> {
    info!("Using tree-sitter parser for language {}", lang);
    match LANGUAGES.get(lang) {
        Some(grammar_fn) => Ok(unsafe { grammar_fn() }),
        None => Err(format_err!("Unsupported language {}", lang)),
    }
}

/// Create an instance of a language from a file extension
///
/// The user may optionally provide a hashmap with overrides
pub fn language_from_ext(
    ext: &str,
    overrides: Option<&HashMap<String, String>>,
) -> Result<Language> {
    if let Some(Some(language_str)) = overrides.map(|x| x.get(ext)) {
        info!(
            "Deduced language \"{}\" from extension \"{}\" provided from user mappings",
            language_str, ext
        );
        return generate_language(language_str);
    };
    let language_str = match FILE_EXTS.get(ext) {
        Some(&language_str) => {
            info!(
                "Deduced language \"{}\" from extension \"{}\" from default mappings",
                language_str, ext
            );
            Ok(language_str)
        }
        None => Err(format_err!("Unsupported filetype \"{}\"", ext)),
    }?;
    generate_language(language_str)
}

/// Parse a file to an AST
///
/// The user may optionally supply the language to use. If the language is not supplied, it will be
/// inferrred from the file's extension.
#[time("info", "parse::{}")]
pub fn parse_file(
    p: &Path,
    language: Option<&str>,
    overrides: Option<&HashMap<String, String>>,
) -> Result<Tree> {
    let text = fs::read_to_string(p)?;
    let mut parser = Parser::new();
    let language = match language {
        Some(x) => {
            info!("Using language {} with parser", x);
            generate_language(x)
        }
        None => {
            if let Some(ext) = p.extension() {
                let ext_str = ext.to_string_lossy();
                language_from_ext(&ext_str, overrides)
            } else {
                Err(format_err!(
                    "Could not deduce an extension for file name \"{}\"",
                    p.to_string_lossy()
                ))
            }
        }
    }?;
    parser
        .set_language(language)
        .map_err(|e| anyhow::format_err!(e))?;
    debug!("Constructed parser");

    match parser.parse(&text, None) {
        Some(ast) => {
            debug!("Parsed AST");
            Ok(ast)
        }
        None => Err(format_err!("Failed to parse file: {}", p.to_string_lossy())),
    }
}

/// Return the languages supported by this instance of the tool in alphabetically sorted order
pub fn supported_languages() -> Vec<&'static str> {
    let mut keys: Vec<&'static str> = LANGUAGES.keys().copied().collect();
    keys.sort_unstable();
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that every parser that this program was compiled to support can be loaded by the tree
    /// sitter [parser](tree_sitter::Parser)
    #[test]
    fn test_loading_languages() {
        // Collect all of the test failures in a vector so we can show a comprehensive error with
        // all of the failed languages instead of panicking one at a time
        let mut failures = Vec::new();

        for (&name, lang) in &LANGUAGES {
            let mut parser = tree_sitter::Parser::new();
            let result = parser.set_language(unsafe { lang() });

            if let Err(e) = result {
                failures.push((name, e));
            }
        }

        assert!(failures.is_empty(), "{:#?}", failures);
    }
}
