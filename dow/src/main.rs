// dow - dev-flow 统一 CLI 调度器
// dow/
// ├── src/
// │   ├── main.rs          -- CLI 入口
// │   ├── cli.rs           -- clap 子命令定义
// │   ├── output.rs        -- JSON / human 输出切换
// │   ├── error.rs         -- 统一错误类型
// │   ├── commands/        -- 子命令实现
// │   ├── hooks/           -- hook 子命令实现
// │   └── lib/             -- 公共库（yaml/version/git 等）

mod cli;
mod commands;
mod core;
mod error;
mod hooks;
mod output;

use clap::Parser;
use cli::{Cli, Commands, HooksCommands};
use std::process;

fn main() {
    let cli = Cli::parse();
    let human = cli.human;

    // 每日版本检查（非 setup/update/self-check 命令时）
    if should_check_version(&cli.command) {
        check_version_background();
    }

    let result = match cli.command {
        Commands::Status(args) => commands::status::run(args, human),
        Commands::Init(args) => commands::init::run(args, human),
        Commands::Check => commands::check::run(human),
        Commands::Claim(args) => commands::claim::run(args, human),
        Commands::Iterate(args) => commands::iterate::run(args, human),
        Commands::Revoke(args) => commands::revoke::run(args, human),
        Commands::Scan => commands::scan::run(human),
        Commands::Validate => commands::validate::run(human),
        Commands::Fix => commands::fix::run(human),
        Commands::Doc(args) => commands::doc::run(args, human),
        Commands::Devtest(args) => commands::devtest::run(args, human),
        Commands::Test(args) => commands::test_runner::run(args, human),
        Commands::Inbox { command } => match command {
            cli::InboxCommands::Context => commands::info::context(),
        },
        Commands::Issue(args) => commands::issue::run(args, human),
        Commands::Version(args) => commands::version::run(args, human),
        Commands::Archive { command } => commands::archive::run(command, human),
        Commands::Hooks {
            codex_hook,
            kiro_hook,
            command,
        } => match command {
            HooksCommands::Context => hooks::context::run(human, codex_hook, kiro_hook),
            HooksCommands::Guard { file } => hooks::guard::run(file.unwrap_or_default(), kiro_hook),
            HooksCommands::PostWrite { file } => hooks::post_write::run(file, kiro_hook),
            HooksCommands::PostBash { command } => hooks::post_bash::run(command, kiro_hook),
            HooksCommands::SaveChangelog => hooks::save_changelog::run(codex_hook, kiro_hook),
        },
        Commands::Setup(args) => commands::setup::run(args.agent, human),
        Commands::Update => commands::update::run(human),
        Commands::SelfCheck => commands::self_check::run(human),
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

    let config = DowConfig::load();

    // 显示缓存的新版本提醒
    if let Some(ref remote) = config.latest_remote_version {
        let current = env!("DOW_VERSION");
        if core::github::compare_versions(current, remote) == std::cmp::Ordering::Less {
            eprintln!(
                "[dow] 新版本 v{} 可用（当前 v{}），运行 `dow update` 升级",
                remote, current
            );
            if let Some(ref notes) = config.latest_release_notes {
                eprintln!("[dow] 变更: {}", notes);
            }
        }
    }

    // 判断是否需要后台检查
    let should_check = match &config.last_version_check {
        None => true,
        Some(last) => chrono::DateTime::parse_from_rfc3339(last)
            .map(|t| {
                let elapsed = chrono::Utc::now().signed_duration_since(t);
                elapsed.num_hours() >= 24
            })
            .unwrap_or(true),
    };

    if should_check {
        // spawn 后台线程，不阻塞主命令
        std::thread::spawn(|| {
            if let Ok(release) = core::github::check_latest_version() {
                let mut config = DowConfig::load();
                config.last_version_check = Some(chrono::Utc::now().to_rfc3339());
                config.latest_remote_version = Some(release.version);
                config.latest_release_notes = release.notes;
                let _ = config.save();
            }
        });
    }
}
