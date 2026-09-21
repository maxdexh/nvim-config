use crate::{
    env::gvim::{api::AutoCmdOpts, lsp::VimLspConfig},
    prelude::*,
};

impl NvimConf<'_> {
    pub fn ft_set_indent(
        &self,
        ft: impl LuaSub<LuaString>,
        indent: impl LuaSub<LuaInt> + 'static + Send + Copy,
    ) {
        self.add_autocmd(
            "FileType",
            mk_builder!(AutoCmdOpts, {
                pattern = Some(ft);
                callback = self.mk_callback(move |conf, ()| {
                    conf.with_vim_opt_local(|opt_local| {
                        tbl!(out(opt_local), {
                            shiftwidth = 0;
                            tabstop = indent;
                            expandtab = true;
                        })
                    })
                    .map(|_| ())
                });
            }),
        );
    }

    pub fn version_range(&self, arg: impl LuaSub<LuaString>) -> Result<LuaVal> {
        self.vim()?.version()?.range()?.call(arg)
    }

    pub fn nix_shell_cmd(
        &self,
        package: &str,
        command: impl IntoIterator<Item: LuaSub<LuaString>>,
    ) -> Result<LuaSeq<LuaString>> {
        let table = self.lua().create_sequence_from([
            "nix", //
            "shell",
            format!("nixpkgs#{package}").as_str(),
            "--command",
        ])?;
        for arg in command {
            table.raw_push_any(arg)?;
        }
        Ok(LuaSeq::cast_table_any(table))
    }

    pub fn config_lsp_noenable(&self, ls: &str, opts: impl LuaSub<LuaStruct<VimLspConfig>>) {
        if self.is_vscode() {
            return;
        }
        do_try(|| self.vim()?.lsp()?.config()?.call((ls, opts))).ok_or_notify(self);
    }
    pub fn enable_lsp(&self, ls: &str) {
        if self.is_vscode() {
            return;
        }
        do_try(|| self.vim()?.lsp()?.enable()?.call(ls)).ok_or_notify(self);
    }
    pub fn config_lsp(&self, ls: &str, opts: impl LuaSub<LuaStruct<VimLspConfig>>) {
        if self.is_vscode() {
            return;
        }
        self.config_lsp_noenable(ls, opts);
        self.enable_lsp(ls);
    }
}
