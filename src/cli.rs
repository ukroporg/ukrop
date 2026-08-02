use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    version,
    about = "Quick directory jumping & command execution",
    after_help = "🇺🇦 Help Ukraine: https://commission.europa.eu/topics/eu-solidarity-ukraine/donate_en"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Interactive directory picker (default)
    Cd {
        /// Pre-fill search query
        #[arg(trailing_var_arg = true)]
        query: Vec<String>,
    },
    /// Interactive command picker
    Run {
        /// Pre-fill search query
        #[arg(trailing_var_arg = true)]
        query: Vec<String>,
    },
    /// Interactive SSH host picker
    Ssh {
        /// Pre-fill search query
        #[arg(trailing_var_arg = true)]
        query: Vec<String>,
    },
    /// Search across all panels (alias for interactive picker with pre-filled query)
    Search {
        /// Search query
        #[arg(trailing_var_arg = true)]
        query: Vec<String>,
    },
    /// Output shell integration script
    Init {
        /// Shell type
        shell: ShellType,
    },
    /// Record directory visit (called by shell hook)
    #[command(hide = true)]
    Hook {
        /// Shell instance id (PID), used to track per-shell directory changes.
        #[arg(long)]
        shell_id: Option<String>,
        #[arg(last = true)]
        path: String,
    },
    /// Record SSH connection (called by shell hook)
    #[command(hide = true)]
    HookSsh {
        /// SSH host args (e.g. "-p 2222 root@myhost" or "myhost")
        #[arg(long, allow_hyphen_values = true)]
        host: String,
        /// Directory the ssh command was issued from.
        #[arg(long)]
        cwd: Option<String>,
    },
    /// Record command execution (called by shell hook)
    #[command(hide = true)]
    HookCmd {
        /// The command that was run
        #[arg(long)]
        cmd: String,
        /// Exit code of the command
        #[arg(long)]
        exit_code: Option<i64>,
        /// Working directory where the command was run
        #[arg(long)]
        cwd: Option<String>,
        /// Command execution duration in milliseconds
        #[arg(long)]
        duration_ms: Option<i64>,
    },
    /// Add favorite directory
    Add {
        /// Path to add
        path: String,
    },
    /// Remove entry from database
    Forget {
        /// Path or command to forget
        path: String,
    },
    /// Import from shell history or restore database from JSONL file
    Import {
        /// Shell type (defaults to auto-detect)
        shell: Option<ShellType>,
        /// Import from JSONL file (full database restore)
        #[arg(long)]
        file: Option<String>,
    },
    /// Export database to JSONL file
    Export {
        /// Output file path (defaults to stdout)
        #[arg(long)]
        file: Option<String>,
    },
    /// Generate demo data for screencasts (replaces current database)
    Demo,
    /// Initial setup: import history, SSH config, add shell integration
    Setup {
        /// Run setup even if already completed
        #[arg(long)]
        force: bool,
    },
    /// Edit configuration
    Config,
    /// List tracked entries
    List {
        /// Show commands instead of directories
        #[arg(long)]
        commands: bool,
        /// Show SSH hosts
        #[arg(long)]
        ssh: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Clone, Debug, clap::ValueEnum)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
    Powershell,
}
