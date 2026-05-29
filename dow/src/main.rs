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

    let result = match cli.command {
        Commands::Status(args) => commands::status::run(args, human),
        Commands::Init(args) => commands::init::run(args, human),
        Commands::Check => commands::check::run(human),
        Commands::Iterate(args) => commands::iterate::run(args, human),
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
        Commands::Hooks { command } => match command {
            HooksCommands::Context => hooks::context::run(human),
            HooksCommands::Guard { file } => hooks::guard::run(file.unwrap_or_default()),
            HooksCommands::PostWrite { file } => hooks::post_write::run(file),
            HooksCommands::PostBash { command } => hooks::post_bash::run(command),
            HooksCommands::SaveChangelog => hooks::save_changelog::run(),
        },
    };

    match result {
        Ok(code) => process::exit(code),
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    }
}
