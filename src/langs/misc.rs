use crate::{env::gvim::lsp::VimLspConfig, prelude::*};

impl NvimConf<'_> {
    pub fn load_shell_langs(&self) {
        if self.is_vscode() {
            return;
        }

        self.config_lsp(
            "bashls",
            mk_builder!(VimLspConfig, {
                cmd = self.nix_shell_cmd("bash-language-server", ["bash-language-server"]);
                filetypes = ["bash", "sh", "zsh"];
            }),
        );
        self.config_lsp(
            "fish_lsp",
            mk_builder!(VimLspConfig, {
                cmd = self.nix_shell_cmd("fish-lsp", ["fish-lsp", "start"]);
            }),
        );
    }
}
