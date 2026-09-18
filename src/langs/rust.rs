use crate::{env::gvim::pack::PackOpts, prelude::*};

impl NvimConf<'_> {
    pub fn load_rust_lang(&self) {
        self.ft_set_indent("rust", 4);
        self.set_formatter("rust", ["rustfmt"]);

        if self.is_vscode() {
            return;
        }

        if let Some(version) = self.version_range("^9").ok_or_notify(self) {
            self.add_packs([mk_builder!(PackOpts, {
                src = "https://github.com/mrcjkb/rustaceanvim";
                version = version;
            })]);
        } else {
            return;
        }

        self.with_vim_g(|g| {
            if let cur_opts = g.get("rustaceanvim")
                && !cur_opts.as_ref().is_ok_and(|it| matches!(it, LuaVal::Nil))
            {
                self.notify(
                    format!("vim.g.rustaceanvim: expected nil, got\n{cur_opts:?}"),
                    NotifyLevel::Warn,
                );
            }
            tbl!(out(g), {
                rustaceanvim = self.rustaceanvim_opts();
            })
        })
        .ok_or_notify(self);
    }

    fn rustaceanvim_opts(&self) -> impl LuaSub<LuaVal> {
        let ra_opts = tbl!(owned, {
            assist = tbl!(owned, {
                preferSelf = true;
            });
        });

        tbl!(owned, {
            server.default_settings = tbl!(owned, {
                "rust-analyzer" = ra_opts;
            });
        })
    }
}
