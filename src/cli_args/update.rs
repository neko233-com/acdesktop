//! `acdesktop update check|apply|auto`

use clap::{Args, Subcommand};

#[derive(Args, Debug)]
#[command(about = "版本检查与自更新")]
pub(crate) struct UpdateArgs {
    #[command(subcommand)]
    pub action: UpdateAction,
}

#[derive(Subcommand, Debug)]
pub(crate) enum UpdateAction {
    /// 检查 GitHub Release 是否有更新版本
    Check,
    /// 下载并安装最新版本（调用 install.sh / install.ps1）
    Apply,
    /// 管理自动更新行为
    #[command(subcommand)]
    Auto(AutoAction),
}

#[derive(Subcommand, Debug)]
pub(crate) enum AutoAction {
    /// 查看自动更新配置
    Status,
    /// 开启自动更新
    On,
    /// 关闭自动更新（~/.acdesktop/config.json auto_update=false）
    Off,
}
