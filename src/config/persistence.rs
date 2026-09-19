use crate::{env::gvim::keymap::KeymapOpts, prelude::*};

crate::utils::from_tbl_proxy!({
    struct Persistence {
        setup: LuaCallable<LuaDict<LuaVal>, ()>,
        load: LuaCallable<Option<LuaDict<LuaVal>>, ()>,
    }
});

impl NvimConf<'_> {
    pub fn req_persistence(&self) -> Result<Persistence> {
        self.setup_plugin::<Persistence>("persistence", |pers| pers.setup()?.call(tbl!(owned, {})))
    }

    pub fn load_persistence(&self) {
        if self.is_vscode() {
            return;
        }

        self.add_packs(["https://github.com/folke/persistence.nvim"]);

        self.req_persistence().ok_or_notify(self);

        self.set_keymap(
            "n",
            "<leader>ul",
            self.mk_callback(|conf, ()| conf.req_persistence()?.load()?.call(())),
            mk_builder!(KeymapOpts, {
                desc = "Load last session (persistence)";
            }),
        );
    }
}
