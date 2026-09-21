use std::{collections::HashSet, sync::RwLock};

use crate::{
    env::{
        NvimConf,
        gvim::{
            api::{AutoCmdOpts, UserCommandArg, UserCommandOpts},
            keymap::KeymapOpts,
            pack::PackOpts,
        },
    },
    prelude::*,
};

impl NvimConf<'_> {
    pub fn vim(&self) -> Result<crate::env::gvim::Vim> {
        self.env().globals.vim()
    }
    pub fn with_vim_opt<T>(
        &self,
        f: impl FnOnce(LuaMapMut<LuaString, LuaVal>) -> Result<T>,
    ) -> Result<T> {
        f(self.vim()?.opt()?)
    }
    pub fn with_vim_opt_local<T>(
        &self,
        f: impl FnOnce(LuaMapMut<LuaString, LuaVal>) -> Result<T>,
    ) -> Result<T> {
        f(self.vim()?.opt_local()?)
    }
    pub fn with_vim_g<T>(
        &self,
        f: impl FnOnce(LuaMapMut<LuaString, LuaVal>) -> Result<T>,
    ) -> Result<T> {
        f(self.vim()?.g()?)
    }
    pub fn add_autocmd(
        &self,
        event: impl LuaSub<LuaString>,
        opts: impl LuaSub<LuaStruct<AutoCmdOpts>>,
    ) {
        do_try(|| {
            self.vim()?
                .api()?
                .nvim_create_autocmd()?
                .call((event, opts))
        })
        .ok_or_notify(self.env());
    }
    pub fn schedule_wrap<A, R>(&self, f: LuaCallable<A, R>) -> Result<LuaCallable<A, R>> {
        self.vim()?
            .schedule_wrap()?
            .as_any()
            .call_any(f.into_any())
            .map(LuaCallable::cast_any_callable)
    }
    pub fn on_very_lazy(&self, f: impl FnOnce(&NvimConf) -> Result<()> + 'static) -> Result<()> {
        let cb = self.mk_once_callback(|conf, ()| f(conf))?;

        let opts = mk_builder!(AutoCmdOpts, {
            callback = self.schedule_wrap(cb);
            once = true;
        });

        self.vim()?
            .api()?
            .nvim_create_autocmd()?
            .call(("UIEnter", opts))
    }
    pub fn run_cmd(&self, cmd: impl LuaSub<LuaString>) {
        do_try(|| self.vim()?.cmd()?.call(cmd)).ok_or_notify(self.env());
    }
    pub fn notify(&self, msg: impl LuaSub<LuaString>, level: NotifyLevel) {
        crate::env::lua_do_notify(self.lua(), msg, level).ok_or_notify(self);
    }
    pub fn set_keymap(
        &self,
        modes: impl LuaSub<LuaUnion<LuaString, LuaSeq<LuaString>>>,
        sequence: impl LuaSub<LuaString>,
        action: impl LuaSub<LuaUnion<LuaString, LuaCallable<(), ()>>>,
        opts: impl LuaSub<LuaStruct<KeymapOpts>>,
    ) {
        do_try(|| {
            self.vim()?
                .keymap()?
                .set()?
                .call((modes, sequence, action, opts))
        })
        .ok_or_notify(self.env());
    }

    pub fn mk_callback<A: FromLuaMultiTyped>(
        &self,
        f: impl Fn(&NvimConf, A) -> Result<()> + 'static,
    ) -> Result<LuaCallable<A, ()>> {
        self.env().create_func(
            move |env, args| {
                f(&env.conf(), args).ok_or_notify(env);
                Ok(())
            },
            |lua, err| {
                Err::<(), _>(err).ok_or_notify(lua);
                Ok(())
            },
        )
    }
    pub fn mk_func<A: FromLuaMultiTyped, R: IntoLuaMultiTyped>(
        &self,
        f: impl Fn(&NvimConf, A) -> Result<R> + 'static,
    ) -> Result<LuaCallable<A, R>> {
        self.env().create_func(
            move |env, args| f(&env.conf(), args),
            |_, err| Err(err.into()),
        )
    }
    pub fn mk_sched_callback<A: FromLuaMultiTyped>(
        &self,
        f: impl Fn(&NvimConf, A) -> Result<()> + 'static,
    ) -> Result<LuaCallable<A, ()>> {
        self.mk_callback(f).and_then(|cb| self.schedule_wrap(cb))
    }
    pub fn mk_once_callback<A: FromLuaMultiTyped>(
        &self,
        f: impl FnOnce(&NvimConf, A) -> Result<()> + 'static,
    ) -> Result<LuaCallable<A, ()>> {
        let func = std::sync::Mutex::new(Some(f));
        self.mk_callback(move |env, args| {
            func.try_lock()
                .map_or_else(
                    |err| match err {
                        std::sync::TryLockError::Poisoned(pe) => Some(pe.into_inner()),
                        std::sync::TryLockError::WouldBlock => None,
                    },
                    Some,
                )
                .and_then(|mut it| it.take())
                .map_or(Ok(()), |f| f(env, args))
        })
    }
    pub fn setup_plugin_now(&self, name: &str, opts: impl LuaSub<LuaDict<LuaVal>>) -> Result<()> {
        if !self.get_setup_state().lock_setup(name) {
            panic!("setup_plugin_now called twice on {name:?}");
        }
        let plugin = self.env().require::<LuaTableAny>(name)?;
        let setup: LuaCallable<LuaDict<LuaVal>, ()> = plugin.get_any("setup")?;
        setup.call(opts)
    }
    pub fn setup_plugin<T: PopLua>(
        &self,
        name: &str,
        setup: impl FnOnce(&mut T) -> Result<()>,
    ) -> Result<T> {
        let mut plugin = self.env().require::<T>(name)?;
        if self.get_setup_state().lock_setup(name) {
            setup(&mut plugin)?;
        }
        Ok(plugin)
    }
    pub fn add_packs(&self, packs: impl LuaSub<LuaSeq<LuaUnion<LuaStruct<PackOpts>, LuaString>>>) {
        do_try(|| self.vim()?.pack()?.add()?.call(packs)).ok_or_notify(self);
    }

    pub fn is_vscode(&self) -> bool {
        struct IsVscode {
            is_vscode: bool,
        }
        self.env()
            .registry
            .get_or_insert(|| IsVscode {
                is_vscode: do_try(|| {
                    Ok(match self.vim()?.g()?.get("vscode")? {
                        LuaVal::Nil | LuaVal::Boolean(false) => false,
                        _ => true,
                    })
                })
                .ok_or_notify(self)
                .unwrap_or_default(),
            })
            .is_vscode
    }
    #[expect(dead_code)]
    pub fn vscode_eval<T: PopLua>(&self, code: &str) -> Result<T> {
        debug_assert!(self.is_vscode());
        self.env().globals.vscode()?.eval()?.call_any_ret(code)
    }

    pub fn create_user_command(
        &self,
        name: impl LuaSub<LuaString>,
        callback: impl LuaSub<LuaCallable<LuaStruct<UserCommandArg>, ()>>,
        opts: impl LuaSub<LuaStruct<UserCommandOpts>>,
    ) -> Result<()> {
        self.vim()?
            .api()?
            .nvim_create_user_command()?
            .call((name, callback, opts))
    }
}

#[derive(Default)]
struct SetupState {
    plugins: RwLock<HashSet<String>>,
}
impl SetupState {
    fn is_setup(&self, name: &str) -> bool {
        self.plugins
            .read()
            .unwrap_or_else(|pe| pe.into_inner())
            .contains(name)
    }
    fn lock_setup(&self, name: &str) -> bool {
        if self.is_setup(name) {
            return false;
        }

        std::hint::cold_path();

        let mut plugins = self.plugins.write().unwrap_or_else(|pe| pe.into_inner());
        if plugins.contains(name) {
            return false;
        }
        plugins.insert(name.into());
        true
    }
}
impl NvimConf<'_> {
    fn get_setup_state(&self) -> std::sync::Arc<SetupState> {
        self.env().registry.get_or_default()
    }
}
