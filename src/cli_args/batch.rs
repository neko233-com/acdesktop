use clap::Parser;

#[derive(Parser, Debug)]
pub(crate) struct BatchArgs {
    #[arg(value_name = "JSON", help = "JSON array of {command, args} objects")]
    pub commands_json: String,
    #[arg(long, help = "Halt the batch on the first failed command")]
    pub stop_on_error: bool,
    #[arg(
        long,
        default_value = "60000",
        help = "Absolute wall-clock budget for the whole non-atomic batch"
    )]
    pub timeout_ms: u64,
    #[arg(
        long,
        help = "跨 app 并行执行：observation 命令（snapshot/find/list-*）天然无副作用，\
                action 命令（click/type/drag）仍按 app 分组串行以保证安全。\
                可配合 --max-workers 控制并发数。"
    )]
    pub parallel: bool,
    #[arg(
        long,
        default_value_t = default_max_workers(),
        help = "--parallel 时最多同时跑的 app 分组数。默认 = CPU 核心数，上限 8。"
    )]
    pub max_workers: usize,
}

fn default_max_workers() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(8)
}
