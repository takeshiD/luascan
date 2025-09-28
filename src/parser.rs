use crate::config::RuntimeVersion;
use full_moon::{LuaVersion, parse_fallible};

#[derive(Debug, Clone)]
pub struct Location {
    pub line_start: usize,
    pub line_end: usize,
    pub col_start: usize,
    pub col_end: usize,
}

#[derive(Debug, Clone)]
pub struct LuascanDiagnostic {
    pub loc: Location,
    pub msg: String,
}

pub fn parse(code: &str, version: RuntimeVersion) -> Vec<LuascanDiagnostic> {
    let version = match version {
        RuntimeVersion::Lua51 => LuaVersion::lua51(),
        RuntimeVersion::Lua52 => LuaVersion::lua52(),
        RuntimeVersion::Lua53 => LuaVersion::lua53(),
        RuntimeVersion::Lua54 => LuaVersion::lua54(),
        _ => panic!("failed version"),
    };
    let ast = parse_fallible(code, version);
    let mut ret = Vec::new();
    for e in ast.errors().iter() {
        match e {
            full_moon::Error::AstError(ast_err) => {
                let range = ast_err.range().clone();
                let loc = Location {
                    line_start: range.0.line(),
                    line_end: range.1.line(),
                    col_start: range.0.character(),
                    col_end: range.1.character(),
                };
                let msg = ast_err.error_message().to_string().clone();
                ret.push(LuascanDiagnostic { loc, msg });
            }
            full_moon::Error::TokenizerError(tkn_err) => {
                let range = tkn_err.range().clone();
                let loc = Location {
                    line_start: range.0.line(),
                    line_end: range.1.line(),
                    col_start: range.0.character(),
                    col_end: range.1.character(),
                };
                let msg = tkn_err.error().to_string();
                ret.push(LuascanDiagnostic { loc, msg });
            }
        }
    }
    ret
}
