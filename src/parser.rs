use crate::config::RuntimeVersion;
use full_moon::{LuaVersion, parse_fallible};

pub fn parse(code: &str, version: RuntimeVersion) -> Vec<String> {
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
                ret.push(ast_err.error_message().to_string().clone());
            }
            full_moon::Error::TokenizerError(tkn_err) => {
                ret.push(tkn_err.error().to_string());
            }
        }
    }
    ret
}
