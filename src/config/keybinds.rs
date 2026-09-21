use crate::{
    env::gvim::{
        api::{UserCommandArg, UserCommandOpts},
        keymap::KeymapOpts,
    },
    prelude::*,
};

fn call_picker(
    conf: &NvimConf,
    name: &str,
    args: impl LuaSub<Option<LuaDict<LuaVal>>>,
) -> Result<()> {
    conf.req_snacks()?.picker()?.get(name)?.call(args)
}
fn get_cwd(conf: &NvimConf) -> Result<LuaString> {
    conf.vim()?.uv()?.cwd()?.call(())
}
fn get_root(conf: &NvimConf) -> Result<LuaString> {
    conf.req_snacks()
        .and_then(|snacks| snacks.git()?.get_root()?.call(()))?
        .map_or_else(|| get_cwd(conf), Ok)
}
impl NvimConf<'_> {
    pub fn load_keybinds(&self) {
        // TODO: Make <C-q> work correctly on vsc

        if !self.is_vscode() {
            self.set_keymap(
                ["i", "n", "s"],
                "<esc>",
                self.mk_func(|conf, ()| {
                    conf.vim()?.cmd()?.call("noh")?;
                    Ok("<esc>")
                }),
                mk_builder!(KeymapOpts, {
                    desc = "Escape and clear hlsearch";
                    expr = true;
                }),
            );
        }

        if !self.is_vscode() {
            self.set_keymap(
                "n",
                "<leader>sx",
                self.mk_callback(|conf, ()| call_picker(conf, "resume", LuaNil)),
                mk_builder!(KeymapOpts, {
                    desc = "Resume Picker";
                }),
            );
        }

        if !self.is_vscode() {
            self.set_keymap(
                "n",
                "<leader>fP",
                self.mk_callback(|env, ()| call_picker(env, "pickers", LuaNil)),
                mk_builder!(KeymapOpts, {
                    desc = "Find Picker";
                }),
            );
        }

        self.set_keymap(
            "n",
            "<leader>ff",
            if self.is_vscode() {
                LuaUnion::Left("<CMD>call VSCodeNotify('workbench.action.quickOpen')<CR>")
            } else {
                LuaUnion::Right(self.mk_callback(|conf, ()| {
                    // calls Snacks.picker.files({ cwd = ... })
                    call_picker(
                        conf,
                        "files",
                        Some(tbl!(owned, {
                            cwd = get_cwd(conf)?;
                        })),
                    )
                }))
            },
            mk_builder!(KeymapOpts, {
                desc = "Find Files (cwd)";
            }),
        );

        if !self.is_vscode() {
            self.set_keymap(
                "n",
                "<leader>fF",
                self.mk_callback(|env, ()| {
                    call_picker(
                        env,
                        "files",
                        Some(tbl!(owned, {
                            cwd = get_root(env)?;
                        })),
                    )
                }),
                mk_builder!(KeymapOpts, {
                    desc = "Find Files (Root Dir)";
                }),
            );
        }

        self.set_keymap(
            "n",
            "<leader>sg",
            if self.is_vscode() {
                LuaUnion::Left("<CMD>call VSCodeNotify('workbench.action.findInFiles')<CR>")
            } else {
                LuaUnion::Right(self.mk_callback(|env, ()| {
                    call_picker(
                        env,
                        "grep",
                        Some(tbl!(owned, {
                            cwd = get_cwd(env)?;
                        })),
                    )
                }))
            },
            mk_builder!(KeymapOpts, {
                desc = "Grep (cwd)";
            }),
        );

        if !self.is_vscode() {
            self.set_keymap(
                "n",
                "<leader>sG",
                self.mk_callback(|env, ()| {
                    call_picker(
                        env,
                        "grep",
                        Some(tbl!(owned, {
                            cwd = get_root(env)?;
                        })),
                    )
                }),
                mk_builder!(KeymapOpts, {
                    desc = "Grep (Root Dir)";
                }),
            );

            self.set_keymap(
                "x",
                "<leader>sg",
                self.mk_callback(|env, ()| {
                    call_picker(
                        env,
                        "grep_word",
                        Some(tbl!(owned, {
                            cwd = get_cwd(env)?;
                        })),
                    )
                }),
                mk_builder!(KeymapOpts, {
                    desc = "Grep Selection (cwd)";
                }),
            );

            self.set_keymap(
                "x",
                "<leader>sG",
                self.mk_callback(|env, ()| {
                    call_picker(
                        env,
                        "grep_word",
                        Some(tbl!(owned, {
                            cwd = get_root(env)?;
                        })),
                    )
                }),
                mk_builder!(KeymapOpts, {
                    desc = "Grep Selection (Root Dir)";
                }),
            );
        }

        if let Some(lspb) = do_try(|| self.vim()?.lsp()?.buf()).ok_or_notify(self) {
            self.set_keymap(
                ["n", "x"],
                "<leader>ca",
                lspb.code_action(),
                mk_builder!(KeymapOpts, {
                    desc = "Code Action";
                }),
            );
            self.set_keymap(
                "n",
                "<leader>cr",
                lspb.rename(),
                mk_builder!(KeymapOpts, {
                    desc = "Rename Symbol";
                }),
            );
            self.set_keymap(
                "n",
                "K",
                lspb.hover(),
                mk_builder!(KeymapOpts, {
                    desc = "Open Symbol Hover";
                }),
            );
            self.set_keymap(
                "i",
                "<C-h>",
                lspb.signature_help(),
                mk_builder!(KeymapOpts, {
                    desc = "Signature Help";
                }),
            );
        }

        // NOTE: This has a builtin bind: <c-w>d
        self.set_keymap(
            "n",
            "<leader>xc",
            do_try(|| self.vim()?.diagnostic()?.open_float()),
            mk_builder!(KeymapOpts, {
                desc = "Show Diagnostic";
            }),
        );

        if !self.is_vscode() {
            self.set_keymap(
                "n",
                "<leader>xx",
                self.mk_callback(|env, ()| call_picker(env, "diagnostics", LuaNil)),
                mk_builder!(KeymapOpts, {
                    desc = "Diagnostics";
                }),
            );
            self.set_keymap(
                "n",
                "<leader>xX",
                self.mk_callback(|env, ()| call_picker(env, "diagnostics_buffer", LuaNil)),
                mk_builder!(KeymapOpts, {
                    desc = "Diagnostics (Buffer)";
                }),
            );
        }

        if !self.is_vscode() {
            self.set_keymap(
                "n",
                "gd",
                self.mk_callback(|conf, ()| call_picker(conf, "lsp_definitions", LuaNil)),
                mk_builder!(KeymapOpts, {
                    desc = "Goto Definition";
                }),
            );
        }

        self.set_keymap(
            "n",
            "gi",
            if self.is_vscode() {
                do_try(|| self.vim()?.lsp()?.buf()?.implementation())
            } else {
                self.mk_callback(|conf, ()| call_picker(conf, "lsp_implementations", LuaNil))
            },
            mk_builder!(KeymapOpts, {
                desc = "Goto Implementations";
            }),
        );

        self.set_keymap(
            "n",
            "gr",
            if self.is_vscode() {
                do_try(|| self.vim()?.lsp()?.buf()?.references())
            } else {
                self.mk_callback(|env, ()| call_picker(env, "lsp_references", LuaNil))
            },
            mk_builder!(KeymapOpts, {
                desc = "Goto References";
            }),
        );

        self.set_keymap(
            "v",
            "<C-c>",
            "\"+y",
            mk_builder!(KeymapOpts, {
                desc = "Copy Selection";
            }),
        );

        if !self.is_vscode() {
            self.set_keymap(
                "t",
                "<ESC><ESC>",
                "<C-\\><C-n>",
                mk_builder!(KeymapOpts, {
                    desc = "Exit Terminal mode";
                }),
            );
        }

        macro_rules! resize {
            ($k:expr, $pref:expr, $v:expr, $desc:expr) => {
                self.set_keymap(
                    "n",
                    concat!("<C-", $k, ">"),
                    concat!("<CMD>", $pref, "resize ", $v, "<CR>"),
                    mk_builder!(KeymapOpts, {
                        desc = $desc;
                    }),
                )
            };
        }
        if !self.is_vscode() {
            resize!("Down", "", "-2", "Decrease window height");
            resize!("Up", "", "+2", "Increase window height");
            resize!("Left", "vertical ", "-2", "Decrease window width");
            resize!("Right", "vertical ", "+2", "Increase window width");
        }

        macro_rules! goto_window {
            ($k:expr, $desc:expr) => {
                self.set_keymap(
                    "n",
                    concat!("<C-", $k, ">"),
                    concat!("<C-w>", $k),
                    mk_builder!(KeymapOpts, {
                        desc = concat!("Go to ", $desc, " window");
                    }),
                )
            };
        }
        // TODO: Reimplement similar in vscode?
        if !self.is_vscode() {
            goto_window!("j", "down");
            goto_window!("k", "up");
            goto_window!("h", "left");
            goto_window!("l", "right");
        }

        macro_rules! diag_jump_base {
            ($kb:expr, $count:expr, $sev:expr, $adj:expr) => {
                self.set_keymap(
                    "n",
                    $kb,
                    if self.is_vscode() {
                        self.mk_callback(|_, ()| {
                            // TODO: Jump to diag
                            Ok(())
                        })
                    } else {
                        self.mk_callback(|conf, ()| {
                            conf.env()
                                .globals
                                .vim()?
                                .diagnostic()?
                                .jump()?
                                .call(tbl!(owned, {
                                    count = $count;
                                    severity = $sev;
                                }))
                        })
                    },
                    mk_builder!(KeymapOpts, {
                        desc = concat!("Go to ", $adj, " ", $sev);
                    }),
                );
            };
        }
        macro_rules! diag_jump {
            ($k:expr, $sev:expr) => {
                diag_jump_base!(concat!("[", $k), -1, $sev, "previous");
                diag_jump_base!(concat!("]", $k), 1, $sev, "next");
            };
        }
        diag_jump!("e", "error");
        diag_jump!("w", "warn");

        self.create_user_command(
            "CopyBufPath",
            self.mk_callback(|conf, opts: LuaStruct<UserCommandArg>| {
                let funcs = conf.vim()?.r#fn()?;

                let args = opts.fargs()?;
                let count = opts.count()?;

                let mut abs = false;
                for i in 0..count {
                    let arg: LuaString = conf.convert(args.get(i)?)?;

                    match &*arg.as_bytes() {
                        b"abs" => abs = true,
                        _ => {
                            return Err(Error::msg(format_args!(
                                "invalid arg {}",
                                String::from_utf8_lossy(&arg.as_bytes())
                            )));
                        }
                    }
                }

                let path: LuaString = funcs.get("expand")?.call_any(
                    if abs { "%" } else { "%:p:." }, //
                )?;

                conf.vim()?.notify()?.call({
                    let mut s = b"Copied to \"+:\n".to_vec();
                    s.extend_from_slice(&path.as_bytes());
                    conf.lua().create_string(s)
                })?;
                funcs.get("setreg")?.call_any::<()>(("+", path))
            }),
            mk_builder!(UserCommandOpts, {}),
        )
        .ok_or_notify(self);
    }
}
