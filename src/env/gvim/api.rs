use crate::prelude::*;

// TODO: Should probably do this via the C api instead.
// Don't use nvim-oxi, it leaks strings

crate::utils::from_tbl_proxy!({
    struct VimApi {
        nvim_create_autocmd: LuaCallable<(LuaString, LuaStruct<AutoCmdOpts>), ()>,
        nvim_set_hl: LuaCallable<(LuaInt, LuaString, LuaStruct<HighlightOpts>), ()>,
        nvim_create_user_command: LuaCallable<
            (
                LuaString,
                LuaCallable<LuaStruct<UserCommandArg>, ()>,
                LuaStruct<UserCommandOpts>,
            ),
            (),
        >,
    }
});

crate::utils::from_tbl_struct!({
    struct UserCommandArg {
        fargs: LuaSeq<LuaVal>,
        count: LuaInt,
    }
});

crate::utils::builder_struct!({
    struct UserCommandOpts {}
});

crate::utils::builder_struct!({
    struct AutoCmdOpts {
        callback: LuaUnion<LuaString, LuaCallable<LuaStruct<AutoCmdArgs>, ()>>,
        once: Option<bool>,
        pattern: Option<LuaString>,
    }
});

crate::utils::from_tbl_struct!({
    struct AutoCmdArgs {
        buf: LuaInt,
        r#match: LuaString,
    }
});

crate::utils::builder_struct!({
    struct HighlightOpts {
        underline: Option<bool>,
        sp: Option<LuaString>,
        link: Option<LuaString>,
        fg: Option<LuaString>,
    }
});
