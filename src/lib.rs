pub mod cli;
pub mod config;
pub mod db;
pub mod frecency;
pub mod history;
pub mod shell;
pub mod ssh;
pub mod tui;
pub mod util;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};

pub fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None => cmd_cd(None),
        Some(Command::Cd { query }) => cmd_cd(query_opt(&query)),
        Some(Command::Run { query }) => cmd_run(query_opt(&query)),
        Some(Command::Ssh { query }) => cmd_ssh(query_opt(&query)),
        Some(Command::Search { query }) => cmd_search(query_opt(&query)),
        Some(Command::Init { shell }) => cmd_init(shell),
        Some(Command::Hook { path }) => cmd_hook(&path),
        Some(Command::HookSsh { host }) => cmd_hook_ssh(&host),
        Some(Command::HookCmd { cmd, exit_code, cwd, duration_ms }) => cmd_hook_cmd(&cmd, exit_code, cwd, duration_ms),
        Some(Command::Add { path }) => cmd_add(&path),
        Some(Command::Forget { path }) => cmd_forget(&path),
        Some(Command::Import { shell }) => cmd_import(shell),
        Some(Command::Setup { force }) => cmd_setup(force),
        Some(Command::Config) => cmd_config(),
        Some(Command::List { commands, ssh, json }) => cmd_list(commands, ssh, json),
    }
}

fn query_opt(parts: &[String]) -> Option<String> {
    let q = parts.join(" ");
    if q.is_empty() { None } else { Some(q) }
}

fn cmd_cd(query: Option<String>) -> Result<()> {
    let db_path = util::db_path()?;
    let mut store = db::store::Store::open(&db_path)?;
    if store.is_empty()? && !setup_marker_exists() {
        eprintln!("Tip: run `ukrop setup` to import shell history, SSH config, and add shell integration.");
        eprintln!();
    }

    // Auto-cleanup stale directories
    let cfg = config::Config::load();
    let _ = store.cleanup_stale_directories(cfg.cleanup.stale_days);

    // Non-interactive mode: if query is provided and stdout is not a TTY, print best match
    if let Some(ref q) = query {
        use std::io::IsTerminal;
        if !std::io::stdout().is_terminal() {
            if let Some(path) = store.best_match_directory(q)? {
                println!("cd:{}", path);
                return Ok(());
            }
            std::process::exit(1);
        }
    }

    drop(store);
    run_tui(tui::PickerMode::Directories, query)
}

fn cmd_run(query: Option<String>) -> Result<()> {
    run_tui(tui::PickerMode::Commands, query)
}

fn cmd_ssh(query: Option<String>) -> Result<()> {
    run_tui(tui::PickerMode::SshHosts, query)
}

fn cmd_search(query: Option<String>) -> Result<()> {
    run_tui(tui::PickerMode::Commands, query)
}

fn run_tui(initial_mode: tui::PickerMode, initial_query: Option<String>) -> Result<()> {
    run_tui_inner(initial_mode, initial_query, false)
}

fn run_tui_inner(initial_mode: tui::PickerMode, initial_query: Option<String>, open_config: bool) -> Result<()> {
    let db_path = util::db_path()?;
    let mut store = db::store::Store::open(&db_path)?;

    match tui::run_picker(initial_mode, &mut store, initial_query, open_config)? {
        Some((mode, selected, edit)) => {
            let prefix = if edit { "edit:" } else { "" };
            match mode {
                tui::PickerMode::Directories => println!("{}cd:{}", prefix, selected),
                tui::PickerMode::Commands => println!("{}run:{}", prefix, selected),
                tui::PickerMode::SshHosts => println!("{}ssh:{}", prefix, selected),
            }
            Ok(())
        }
        None => std::process::exit(130),
    }
}

fn cmd_config() -> Result<()> {
    run_tui_inner(tui::PickerMode::Directories, None, true)
}

fn cmd_init(shell: cli::ShellType) -> Result<()> {
    match shell {
        cli::ShellType::Bash => print!("{}", shell::bash::init_script()),
        cli::ShellType::Zsh => print!("{}", shell::zsh::init_script()),
        cli::ShellType::Fish => print!("{}", shell::fish::init_script()),
        cli::ShellType::Powershell => print!("{}", shell::powershell::init_script()),
    }
    Ok(())
}

fn cmd_hook(path: &str) -> Result<()> {
    let db_path = util::db_path()?;
    let mut store = db::store::Store::open(&db_path)?;
    store.record_visit(path)?;
    Ok(())
}

fn cmd_hook_ssh(host: &str) -> Result<()> {
    let db_path = util::db_path()?;
    let mut store = db::store::Store::open(&db_path)?;
    store.record_ssh_host(host, None, None, None, "hook")?;
    Ok(())
}

fn cmd_hook_cmd(cmd: &str, exit_code: Option<i64>, cwd: Option<String>, duration_ms: Option<i64>) -> Result<()> {
    let cfg = config::Config::load();
    if config::should_ignore(&cfg, cmd) {
        return Ok(());
    }

    let db_path = util::db_path()?;
    let mut store = db::store::Store::open(&db_path)?;
    store.record_command_full(cmd, "hook", exit_code, cwd.as_deref(), duration_ms)?;

    // Also record SSH host when user runs ssh directly
    let trimmed = cmd.trim();
    if trimmed.starts_with("ssh ") && !trimmed.starts_with("ssh-") {
        let args = &trimmed[4..];
        let _ = store.record_ssh_from_command(args.trim());
    }

    Ok(())
}

fn cmd_add(path: &str) -> Result<()> {
    let resolved = util::resolve_path(path)?;
    if !std::path::Path::new(&resolved).is_dir() {
        anyhow::bail!("Directory does not exist: {}", resolved);
    }
    let db_path = util::db_path()?;
    let mut store = db::store::Store::open(&db_path)?;
    store.add_favorite(&resolved)?;
    eprintln!("Added favorite: {}", resolved);
    Ok(())
}

fn cmd_forget(path: &str) -> Result<()> {
    let db_path = util::db_path()?;
    let mut store = db::store::Store::open(&db_path)?;
    let removed = store.forget(path)?;
    if removed {
        eprintln!("Removed: {}", path);
    } else {
        eprintln!("Not found: {}", path);
    }
    Ok(())
}

fn cmd_import(shell: Option<cli::ShellType>) -> Result<()> {
    let db_path = util::db_path()?;
    let mut store = db::store::Store::open(&db_path)?;

    let shell = shell.unwrap_or_else(|| detect_shell());

    let commands = match shell {
        cli::ShellType::Bash => history::bash::parse_history_with_cwd()?,
        cli::ShellType::Zsh => history::zsh::parse_history_with_cwd()?,
        cli::ShellType::Fish => history::fish::parse_history_with_cwd()?,
        cli::ShellType::Powershell => history::powershell::parse_history_with_cwd()?,
    };

    let cmd_count = commands.len();
    store.import_commands_with_cwd_batch(&commands, "history")?;

    let directories = match shell {
        cli::ShellType::Bash => history::bash::extract_directories_from_history()?,
        cli::ShellType::Zsh => history::zsh::extract_directories_from_history()?,
        cli::ShellType::Fish => history::fish::extract_directories_from_history()?,
        cli::ShellType::Powershell => history::powershell::extract_directories_from_history()?,
    };

    let dir_count = directories.len();
    store.import_visits_batch(&directories)?;

    let ssh_config_hosts = ssh::config::parse_ssh_config().unwrap_or_default();
    let ssh_config_count = ssh_config_hosts.len();
    store.import_ssh_hosts_batch(&ssh_config_hosts, "config")?;

    let ssh_history_hosts = match shell {
        cli::ShellType::Bash => history::bash::extract_ssh_hosts_from_history()?,
        cli::ShellType::Zsh => history::zsh::extract_ssh_hosts_from_history()?,
        cli::ShellType::Fish => history::fish::extract_ssh_hosts_from_history()?,
        cli::ShellType::Powershell => history::powershell::extract_ssh_hosts_from_history()?,
    };
    let ssh_hist_count = ssh_history_hosts.len();
    store.import_ssh_hosts_batch(&ssh_history_hosts, "history")?;

    eprintln!(
        "Imported {} commands, {} directories, {} SSH hosts (config) + {} SSH hosts (history) from {:?}",
        cmd_count, dir_count, ssh_config_count, ssh_hist_count, shell
    );
    Ok(())
}

fn format_score(score: f64) -> String {
    format!("{}", score.round() as u64)
}

fn setup_marker_path() -> Option<std::path::PathBuf> {
    util::data_dir().ok().map(|d| d.join(".setup_done"))
}

fn import_marker_path() -> Option<std::path::PathBuf> {
    util::data_dir().ok().map(|d| d.join(".import_done"))
}

fn import_recently_done() -> bool {
    import_marker_path()
        .and_then(|p| std::fs::metadata(&p).ok())
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.elapsed().ok())
        .map(|elapsed| elapsed.as_secs() < 3600)
        .unwrap_or(false)
}

fn write_import_marker() {
    if let Some(p) = import_marker_path() {
        let _ = std::fs::write(&p, "");
    }
}

fn setup_marker_exists() -> bool {
    setup_marker_path().map(|p| p.exists()).unwrap_or(false)
}

fn write_setup_marker() {
    if let Some(p) = setup_marker_path() {
        let _ = std::fs::write(&p, "");
    }
}

fn detect_shell() -> cli::ShellType {
    if std::env::var("ZSH_VERSION").is_ok()
        || std::env::var("SHELL").map(|s| s.contains("zsh")).unwrap_or(false)
    {
        cli::ShellType::Zsh
    } else if std::env::var("FISH_VERSION").is_ok()
        || std::env::var("SHELL").map(|s| s.contains("fish")).unwrap_or(false)
    {
        cli::ShellType::Fish
    } else if std::env::var("PSModulePath").is_ok() && std::env::var("SHELL").is_err() {
        cli::ShellType::Powershell
    } else {
        cli::ShellType::Bash
    }
}

fn shell_rc_path(shell: &cli::ShellType) -> Option<std::path::PathBuf> {
    let home = dirs::home_dir()?;
    Some(match shell {
        cli::ShellType::Zsh => home.join(".zshrc"),
        cli::ShellType::Bash => {
            let bashrc = home.join(".bashrc");
            if bashrc.exists() {
                bashrc
            } else {
                home.join(".bash_profile")
            }
        }
        cli::ShellType::Fish => home.join(".config/fish/config.fish"),
        cli::ShellType::Powershell => {
            // PowerShell profile: ~/Documents/PowerShell/Microsoft.PowerShell_profile.ps1 (pwsh)
            // or ~/Documents/WindowsPowerShell/Microsoft.PowerShell_profile.ps1 (Windows PS)
            let ps_dir = home.join("Documents/PowerShell");
            if ps_dir.exists() {
                ps_dir.join("Microsoft.PowerShell_profile.ps1")
            } else {
                home.join(".config/powershell/Microsoft.PowerShell_profile.ps1")
            }
        }
    })
}

fn shell_init_line(shell: &cli::ShellType) -> &'static str {
    match shell {
        cli::ShellType::Zsh => r#"eval "$(ukrop init zsh)""#,
        cli::ShellType::Bash => r#"eval "$(ukrop init bash)""#,
        cli::ShellType::Fish => "ukrop init fish | source",
        cli::ShellType::Powershell => r#"Invoke-Expression (& { (ukrop init powershell | Out-String) })"#,
    }
}

fn ask_yes_no(prompt: &str) -> bool {
    use std::io::Write;
    eprint!("{} [Y/n] ", prompt);
    std::io::stderr().flush().ok();
    let mut input = String::new();
    if let Ok(tty) = std::fs::File::open("/dev/tty") {
        use std::io::BufRead;
        let mut reader = std::io::BufReader::new(tty);
        reader.read_line(&mut input).ok();
    } else {
        std::io::stdin().read_line(&mut input).ok();
    }
    let trimmed = input.trim().to_lowercase();
    trimmed.is_empty() || trimmed == "y" || trimmed == "yes"
}

fn cmd_setup(force: bool) -> Result<()> {
    if !force && setup_marker_exists() {
        eprintln!("Setup already completed. Use `ukrop setup --force` to run again.");
        return Ok(());
    }

    let shell = detect_shell();
    eprintln!("Detected shell: {:?}", shell);

    eprintln!();
    if import_recently_done() {
        eprintln!("Import was done less than an hour ago, skipping.");
    } else if ask_yes_no("Import shell history and SSH config?") {
        cmd_import(Some(shell.clone()))?;
        write_import_marker();
    } else {
        eprintln!("Skipping import.");
    }

    eprintln!();
    let init_line = shell_init_line(&shell);
    if let Some(rc_path) = shell_rc_path(&shell) {
        let rc_content = std::fs::read_to_string(&rc_path).unwrap_or_default();
        if rc_content.contains("ukrop init") {
            eprintln!("Shell integration already in {}", rc_path.display());
        } else if ask_yes_no(&format!(
            "Add shell integration to {}?\n  {}",
            rc_path.display(),
            init_line
        )) {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(&rc_path)?;
            writeln!(f)?;
            writeln!(f, "# ukrop - quick directory jumping & command execution")?;
            writeln!(f, "{}", init_line)?;
            eprintln!("Added to {}", rc_path.display());
            eprintln!("Run `source {}` or restart your shell to activate.", rc_path.display());
        } else {
            eprintln!("Skipping. You can add it manually:");
            eprintln!("  {}", init_line);
        }
    }

    write_setup_marker();
    eprintln!();
    eprintln!("Setup complete! The `u` shortcut is available after shell integration is loaded.");

    if let Some(rc_path) = shell_rc_path(&shell) {
        let rc_content = std::fs::read_to_string(&rc_path).unwrap_or_default();
        if !rc_content.contains("ukrop-dev") {
            eprintln!();
            eprintln!("Tip: add this to {} for a dev rebuild shortcut:", rc_path.display());
            eprintln!();
            eprintln!("  ukrop-dev() {{");
            eprintln!("      cargo install --path . --force && source {} && ukrop setup --force", rc_path.display());
            eprintln!("  }}");
        }
    }
    Ok(())
}

fn cmd_list(commands: bool, ssh: bool, json: bool) -> Result<()> {
    let db_path = util::db_path()?;
    let mut store = db::store::Store::open(&db_path)?;

    if ssh {
        let entries = store.list_ssh_hosts()?;
        if json {
            println!("{}", serde_json::to_string_pretty(&entries)?);
        } else {
            for e in &entries {
                let fav = if e.is_favorite { " *" } else { "" };
                println!("{:>8}  {}{}", format_score(e.score), e.host, fav);
            }
        }
    } else if commands {
        let entries = store.list_commands()?;
        if json {
            println!("{}", serde_json::to_string_pretty(&entries)?);
        } else {
            for e in &entries {
                let fav = if e.is_favorite { " *" } else { "" };
                println!("{:>8}  {}{}", format_score(e.score), e.command, fav);
            }
        }
    } else {
        let entries = store.list_directories()?;
        if json {
            println!("{}", serde_json::to_string_pretty(&entries)?);
        } else {
            for e in &entries {
                let fav = if e.is_favorite { " *" } else { "" };
                println!("{:>8}  {}{}", format_score(e.score), e.path, fav);
            }
        }
    }
    Ok(())
}
