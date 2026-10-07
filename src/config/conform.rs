use crate::{env::gvim::keymap::KeymapOpts, prelude::*};

crate::utils::from_tbl_proxy!({
    struct Conform {
        setup: LuaCallable<LuaDict<LuaVal>, ()>,
        formatters_by_ft: LuaDictMut<LuaSeq<LuaString>>,
        formatters: LuaDictMut<LuaDictMut<LuaVal>>,
    }
});

impl NvimConf<'_> {
    pub fn load_conform(&self) {
        self.add_packs(["https://github.com/stevearc/conform.nvim"]);
    }
    fn req_conform(&self) -> Result<Conform> {
        use std::sync::atomic::{AtomicBool, Ordering};
        #[derive(Default)]
        struct EnableFmt {
            disabled: AtomicBool,
        }
        let fmt_on_save = {
            let opts_tbl = tbl!(owned, {
                timeout_ms = 1000;
                lsp_format = "fallback";
            })
            .eval(self)?;
            self.mk_func(move |conf, ()| {
                let enabled = !conf
                    .env()
                    .registry
                    .get_or_default::<EnableFmt>()
                    .disabled
                    .load(Ordering::Relaxed);
                Ok(enabled.then(|| opts_tbl.clone()))
            })?
        };
        let toggle_fmt_on_save = self.mk_callback(|conf, ()| {
            let enabled = conf
                .env()
                .registry
                .get_or_default::<EnableFmt>()
                .disabled
                .fetch_not(Ordering::Relaxed);

            conf.notify(
                if enabled {
                    "enabled format on save"
                } else {
                    "disabled format on save"
                },
                NotifyLevel::Info,
            );

            Ok(())
        });
        self.set_keymap(
            "n",
            "<leader>uf",
            toggle_fmt_on_save,
            mk_builder!(KeymapOpts, {
                desc = "Toggle Formatter";
            }),
        );

        self.setup_plugin::<Conform>("conform", |conform| {
            conform.setup()?.call(tbl!(owned, {
                format_on_save = fmt_on_save;
            }))
        })
    }

    // FIXME: table arg can be more than sequence, e.g. fallback = ...
    pub fn set_formatter(&self, ft: impl LuaSub<LuaString>, table: impl LuaSub<LuaSeq<LuaString>>) {
        do_try(|| {
            let conform = self.req_conform()?;
            conform.formatters_by_ft()?.set(ft, table)?;

            Ok(())
        })
        .ok_or_notify(self);
    }
    pub fn formatter_use_nix(&self, formatter: &str, package: &str, cmd: &str) {
        let flake = std::sync::Arc::<str>::from(format!("nixpkgs#{package}"));

        // HACK: Prebuild the flake so that conform doesn't timeout later
        {
            let flake = flake.clone();
            std::thread::spawn(move || {
                std::process::Command::new("nix")
                    .arg("build")
                    .arg("--no-link")
                    .arg(&*flake)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                    .unwrap()
                    .wait()
                    .unwrap();
            });
        }

        do_try(|| {
            let lua = self.lua();

            let formatter: LuaString = lua_conv_sub(lua, formatter)?;

            let fmts = self.req_conform()?.formatters()?.into_table_any();
            let settings = match fmts.get_any(formatter.clone())? {
                Some(s) => s,
                None => {
                    let s = lua.create_table()?;
                    fmts.set_any(formatter, s.clone())?;
                    s
                }
            };
            settings.set_any("command", "nix")?;
            let prepend_args = self.lua().create_sequence_from([
                "shell",
                &flake,
                "--command",
                cmd, //
            ])?;
            let key = lua.create_string("prepend_args")?;
            if let Some(t) = settings.get_any::<Option<LuaTableAny>>(key.clone())? {
                for v in t.sequence_values::<mlua::Value>() {
                    prepend_args.raw_push_any(v?)?;
                }
            }
            settings.set_any(key, prepend_args)
        })
        .ok_or_notify(self);
    }
}
