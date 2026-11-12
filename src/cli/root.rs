use std::path::PathBuf;

use clap::Parser;

use super::{Commands, post_action_wait::PostActionWaitArgs};
use crate::cli_args::interaction::InteractionArgs;

const BEFORE_HELP: &str = include_str!("help_before.txt");
const AFTER_HELP: &str = include_str!("help_after.txt");

#[derive(Parser, Debug)]
#[command(
    name = "acdesktop",
    version,
    about = "Reliable computer use for AI agents — see and operate desktop apps",
    long_about = None,
    before_help = BEFORE_HELP,
    after_help = AFTER_HELP,
)]
pub(crate) struct Cli {
    #[command(flatten)]
    pub visual_debug: crate::visual_debug::options::DebugOptions,
    #[command(flatten)]
    pub identity: super::identity::IdentityArgs,
    #[arg(
        long,
        global = true,
        help = "Append reliability trace JSONL to this path"
    )]
    pub trace: Option<PathBuf>,
    #[arg(
        long,
        global = true,
        help = "Fail on trace setup/pre-action write errors"
    )]
    pub trace_strict: bool,
    #[command(flatten)]
    pub interaction: InteractionArgs,
    #[command(flatten)]
    pub post_action_wait: PostActionWaitArgs,
    #[arg(
        long,
        global = true,
        help = "极快模式：跳过 post_action_wait、减少内部等待、使用短超时。\
               也可在 ~/.acdesktop/config.json 里设 fast_mode: true"
    )]
    pub fast: bool,
    #[command(subcommand)]
    pub command: Option<Commands>,
}

impl Cli {
    /// 合并命令行 `--fast` 与配置文件 `fast_mode`
    pub(crate) fn fast_mode(&self) -> bool {
        if self.fast { return true; }
        crate::config::UserConfig::load().fast_mode
    }
}
