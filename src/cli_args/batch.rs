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
        help = "Run observations concurrently. Semantic ref actions can run across verified process identities; global and headed operations run in order."
    )]
    pub parallel: bool,
    #[arg(
        long,
        default_value_t = default_max_workers(),
        help = "Maximum concurrent groups with --parallel (default: CPU count, capped at 8)"
    )]
    pub max_workers: usize,
}

fn default_max_workers() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(8)
}
