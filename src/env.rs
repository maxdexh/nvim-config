mod plugin;

use crate::{lua::AsLua, prelude::*};

pub mod gvim;
pub mod vscode;

crate::utils::from_tbl_proxy!({
    struct Globals {
        vim: gvim::Vim,
        vscode: vscode::Vscode,
        require: LuaCallable<LuaString, LuaVal>,
    }
});

#[derive(Clone, Debug)]
pub struct Nvim {
    pub lua: Lua,
    pub globals: Globals,
    pub registry: crate::registry::Registry,
}

pub enum NotifyLevel {
    Error,
    Warn,
    Info,
}

pub fn lua_do_notify(lua: &Lua, msg: impl LuaSub<LuaString>, level: NotifyLevel) -> Result<()> {
    let vim = lua.convert::<Globals>(lua.globals())?.vim()?;
    vim.notify()?.call((msg, {
        let levels = vim.log()?.levels()?;
        match level {
            NotifyLevel::Error => levels.ERROR()?,
            NotifyLevel::Warn => levels.WARN()?,
            NotifyLevel::Info => levels.INFO()?,
        }
    }))
}

#[cold]
pub fn lua_notify_err(lua: Option<&Lua>, err: impl std::fmt::Display) {
    let mut msg_begin = format!(
        "{err}\n\n{}\n{}\n\n",
        if cfg!(debug_assertions) {
            ""
        } else {
            "WARN: backtrace may not include all info in release mode"
        },
        std::backtrace::Backtrace::force_capture()
    );
    let Some(lua) = lua else {
        msg_begin.push_str("No lua env was available");
        eprintln!("{msg_begin}");
        return;
    };
    let msg = lua
        .as_mlua()
        .traceback(Some(&msg_begin), 0)
        .map_err(|tb_err| {
            std::fmt::write(
                &mut msg_begin,
                format_args!("failed to create lua trackback: {tb_err}",),
            )
            .unwrap_or_else(|_| unreachable!("error in infallible write"));
            msg_begin
        })
        .map_or_else(mlua::Either::Left, mlua::Either::Right);

    () = lua_do_notify(
        lua,
        msg.as_ref().map_left(std::ops::Deref::deref),
        NotifyLevel::Error,
    )
    .unwrap_or_else(|notify_err| {
        eprintln!("Failed to notify: {notify_err}\n");
        match msg {
            mlua::Either::Left(l) => eprintln!("{l}"),
            mlua::Either::Right(r) => eprintln!("{}", String::from_utf8_lossy(&r.as_bytes())),
        }
    });
}

impl Nvim {
    pub fn create_func<A: FromLuaMultiTyped, R: IntoLuaMultiTyped>(
        &self,
        f: impl Fn(&Nvim, A) -> Result<R> + 'static,
        err_handler: impl Fn(&Lua, Error) -> mlua::Result<R::IntoReprMulti> + 'static,
    ) -> Result<LuaCallable<A, R>> {
        let env = self.clone();
        self.lua
            .as_mlua()
            .create_function(move |lua, args| {
                A::from_mlua_multi(args)
                    .and_then(|args| f(&env, args).and_then(|it| it.into_mlua_multi()))
                    .or_else(|err| err_handler(lua.lua(), err))
            })
            .map(LuaCallable::from_mlua_func)
            .map_err(Into::into)
    }

    pub fn require<T: PopLua>(&self, name: impl LuaSub<LuaString>) -> Result<T> {
        self.globals.require()?.call_any_ret(name)
    }
}
impl AsLua for Nvim {
    fn lua(&self) -> &Lua {
        &self.lua
    }
}
impl AsLua for NvimConf<'_> {
    fn lua(&self) -> &Lua {
        &self.env().lua
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NvimConf<'a>(&'a crate::prelude::Nvim);

impl crate::prelude::Nvim {
    pub fn conf(&self) -> NvimConf<'_> {
        NvimConf(self)
    }
}
impl NvimConf<'_> {
    pub fn env(&self) -> &crate::prelude::Nvim {
        self.0
    }
}
