use crate::prelude::*;

impl NvimConf<'_> {
    pub fn load_options(&self) {
        self.with_vim_opt(|opt| {
            tbl!(out(opt), {
                shiftwidth = 0;
                tabstop = 2;
                expandtab = true;
                number = true;
                relativenumber = true;
                undofile = true;
                fillchars = tbl!(owned, {
                    eob = " ";
                });
                // TODO: format-based override
                wrap = false;

                // This is done by lazyvim and makes <c-o> jump between buffers.
                // As this is part of my muscle memory, keep it for now.
                jumpoptions = "view";

                // make search case insensitive unless you add \C or an uppercase letter
                ignorecase = true;
                smartcase = true;

                // put splits on the correct sides (why is the default splitabove, wtf)
                splitright = true;
                splitbelow = true;

                secure = true;
                exrc = true;
            })
        })
        .ok_or_notify(self);

        self.with_vim_g(|g| {
            tbl!(out(g), {
                snacks_animate = false;
                mapleader = " ";
            })
        })
        .ok_or_notify(self);

        do_try(|| {
            self.env()
                .globals
                .vim()?
                .diagnostic()?
                .config()?
                .call(tbl!(owned, {
                    virtual_text.severity.min = 2; // 2 = warn
                }))
        })
        .ok_or_notify(self);
    }
}
