//! `acdesktop config get|set|path`

use clap::{Args, Subcommand};

#[derive(Args, Debug)]
#[command(about = "查看或修改用户配置 (~/.acdesktop/config.json)")]
pub(crate) struct ConfigArgs {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Subcommand, Debug)]
pub(crate) enum ConfigAction {
    /// 打印完整配置
    Show,
    /// 打印配置文件路径
    Path,
    /// 设置单个字段：set fast_mode true | set auto_update false
    Set { key: String, value: String },
}
