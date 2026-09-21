use crate::prelude::*;

pub mod api;
pub mod diagnostic;
pub mod keymap;
pub mod log;
pub mod lsp;
pub mod pack;
pub mod treesitter;
pub mod uv;
pub mod version;

crate::utils::from_tbl_proxy!({
    struct Vim {
        opt: LuaDictMut<LuaVal>,
        opt_local: LuaDictMut<LuaVal>,
        g: LuaDictMut<LuaVal>,
        uv: uv::VimUV,
        pack: pack::VimPack,
        keymap: keymap::VimKeymap,
        diagnostic: diagnostic::VimDiagnostic,
        notify: LuaCallable<(LuaString, LuaInt), ()>,
        cmd: LuaCallable<LuaString, ()>,
        version: version::VimVersion,
        api: api::VimApi,
        log: log::VimLog,
        lsp: lsp::VimLsp,
        schedule_wrap: LuaCallable<crate::lua::LuaCallableAny, crate::lua::LuaCallableAny>,
        treesitter: treesitter::VimTreesitter,
        r#fn: LuaMap<LuaString, crate::lua::LuaCallableAny>,
    }
});
