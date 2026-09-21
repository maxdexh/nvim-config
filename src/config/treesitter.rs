use crate::{
    env::gvim::api::{AutoCmdArgs, AutoCmdOpts},
    prelude::*,
};

crate::utils::from_tbl_proxy!({
    struct TSPlugin {
        setup: LuaCallable<LuaDict<LuaVal>, ()>,
    }
});

impl NvimConf<'_> {
    fn req_ts(&self) -> Result<TSPlugin> {
        self.setup_plugin::<TSPlugin>("nvim-treesitter", |ts| {
            ts.setup()?.call(tbl!(owned, {
                auto_install = true;
                highlight.enable = true;
                indent = tbl!(owned, {
                    enable = true;
                    disable = ["python", "css", "rust"];
                });
            }))
        })
    }
    pub fn load_treesitter(&self) {
        if self.is_vscode() {
            return;
        }
        self.add_packs(["https://github.com/nvim-treesitter/nvim-treesitter"]);

        self.on_very_lazy(|conf| {
            conf.req_ts()?;
            Ok(())
        })
        .ok_or_notify(self);

        let cb = self.mk_callback(|conf, args: LuaStruct<AutoCmdArgs>| {
            let ft = args.r#match()?;
            let vts = conf.vim()?.treesitter()?;

            let Some(lang) = vts.language()?.get_lang()?.call(ft)? else {
                return Ok(());
            };

            if vts.query()?.get()?.call((lang, "highlights"))?.is_none() {
                return Ok(());
            }

            vts.start()?.call(args.buf()?)
        });

        self.add_autocmd(
            "FileType",
            mk_builder!(AutoCmdOpts, {
                callback = cb;
            }),
        );
    }
}
