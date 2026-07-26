// dow - dev-flow unified CLI dispatcher
// dow/
// ├── src/
// │   ├── main.rs          -- CLI entry point
// │   ├── cli.rs           -- clap subcommand definitions
// │   ├── output.rs        -- JSON / human output toggle
// │   ├── error.rs         -- Unified error type
// │   ├── commands/        -- Subcommand implementations
// │   ├── hooks/           -- Hook subcommand implementations
// │   └── core/            -- Common library (yaml/version/git, etc.)

mod cli;
mod commands;
mod core;
mod dashboard;
mod error;
mod hooks;
mod output;

use clap::Parser;
use cli::{Cli, Commands, HooksCommands};
use std::process;

fn main() {
    let cli = Cli::parse();
    let human = cli.human;

    if should_check_version(&cli.command) {
        check_version_background();
    }

    let result = match cli.command {
        Commands::Task { command } => commands::task::run(command, human),
        Commands::Issue { command } => commands::issue::run(command, human),
        Commands::Changelog { command } => commands::changelog_cmd::run(command, human),
        Commands::Brainstorm { command } => commands::brainstorm::run(command, human),
        Commands::Prd { command } => commands::prd::run(command, human),
        Commands::Spec { command } => commands::spec_cmd::run(command, human),
        Commands::Status(args) => commands::status::run(args, human),
        Commands::Init(args) => commands::init::run(args, human),
        Commands::Doctor(args) => commands::doctor::run(args, human),
        Commands::Fix => commands::doctor::run(cli::DoctorArgs { fix: true }, human),
        Commands::Iterate(args) => commands::iterate::run(args, human),
        Commands::Scan => commands::scan::run(human),
        Commands::Test(args) => commands::test_runner::run(args, human),
        Commands::Inbox { command } => match command {
            cli::InboxCommands::Context => commands::inbox::context(),
        },
        Commands::Version(args) => commands::version::run(args, human),
        Commands::Archive { command } => commands::archive::run(command, human),
        Commands::Hooks {
            codex_hook,
            kiro_hook,
            command,
        } => match command {
            HooksCommands::Context => hooks::context::run(human, codex_hook, kiro_hook),
            HooksCommands::Guard { file } => hooks::guard::run(file.unwrap_or_default(), kiro_hook),
            HooksCommands::PostWrite { file } => {
                hooks::post_write::run(file, codex_hook, kiro_hook)
            }
            HooksCommands::PostBash { command } => {
                hooks::post_bash::run(command, codex_hook, kiro_hook)
            }
            HooksCommands::SessionStop => hooks::session_stop::run(codex_hook, kiro_hook),
        },
        Commands::Rollback(args) => commands::rollback::run(args, human),
        Commands::Claim(args) => commands::claim::run(args, human),
        Commands::Setup(args) => commands::setup::run(args.agent, human),
        Commands::Update => commands::update::run(human),
        Commands::SelfCheck => commands::self_check::run(human),
        Commands::Dashboard(args) => commands::dashboard::run(args, human),
    };

    match result {
        Ok(code) => process::exit(code),
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    }
}

fn should_check_version(cmd: &Commands) -> bool {
    !matches!(
        cmd,
        Commands::Setup(_)
            | Commands::Update
            | Commands::SelfCheck
            | Commands::Hooks { .. }
            | Commands::Version(_)
    )
}

fn check_version_background() {
    use core::config::DowConfig;

    let mut config = DowConfig::load();
    if !version_check_cache_is_fresh(&config) {
        sync_latest_release_cache(&mut config);
    }

    let current = env!("DOW_VERSION");
    if let (Some(remote), Some(published_at)) = (
        config.latest_remote_version.as_deref(),
        config.latest_remote_published_at.as_deref(),
    ) {
        if core::github::is_update_available(current, remote, published_at) {
            eprintln!(
                "[dow] New version v{} available (current v{}), run `dow update` to upgrade",
                remote, current
            );
            if let Some(ref notes) = config.latest_release_notes {
                eprintln!("[dow] Changes: {}", notes);
            }
        }
    }
}

fn version_check_cache_is_fresh(config: &core::config::DowConfig) -> bool {
    if config.latest_remote_version.is_none() || config.latest_remote_published_at.is_none() {
        return false;
    }
    config
        .last_version_check
        .as_deref()
        .and_then(|last| chrono::DateTime::parse_from_rfc3339(last).ok())
        .map(|t| {
            let elapsed = chrono::Utc::now().signed_duration_since(t);
            elapsed.num_hours() < 24
        })
        .unwrap_or(false)
}

fn sync_latest_release_cache(config: &mut core::config::DowConfig) {
    match core::github::check_latest_version() {
        Ok(release) => {
            config.last_version_check = Some(chrono::Utc::now().to_rfc3339());
            config.latest_remote_version = Some(release.version);
            config.latest_remote_published_at = Some(release.published_at);
            config.latest_release_notes = release.notes;
        }
        Err(_) => {
            config.last_version_check = None;
            config.latest_remote_version = None;
            config.latest_remote_published_at = None;
            config.latest_release_notes = None;
        }
    }
    let _ = config.save();
}
