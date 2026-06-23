// dow/src/
// ├── cli.rs  -- clap subcommand and parameter definitions

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "dow", about = "dev-flow unified CLI dispatcher", version = env!("DOW_VERSION"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Human-friendly output (default JSON)
    #[arg(short = 'H', long = "human", global = true)]
    pub human: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Read/write STATUS.yaml
    Status(StatusArgs),

    /// Initialize dev-flow workflow management
    Init(InitArgs),

    /// Document spec check
    Check,

    /// Iteration delivery
    Iterate(IterateArgs),

    /// Project scan
    Scan,

    /// Validate .dev-doc structure
    Validate,

    /// Fix .dev-doc file format issues
    Fix,

    /// Generate document template
    Doc(DocArgs),

    /// Task-level testing
    Devtest(DevtestArgs),

    /// Run full test suite
    Test(TestArgs),

    /// Internal common library
    Inbox {
        #[command(subcommand)]
        command: InboxCommands,
    },

    /// Issue management
    Issue(IssueArgs),

    /// Read/write VERSION
    Version(VersionArgs),

    /// Archive query
    Archive {
        #[command(subcommand)]
        command: ArchiveCommands,
    },

    /// Hook subcommands
    Hooks {
        /// Output Codex hook protocol JSON
        #[arg(long, global = true)]
        codex_hook: bool,

        /// Output Kiro hook protocol JSON
        #[arg(long, global = true)]
        kiro_hook: bool,

        #[command(subcommand)]
        command: HooksCommands,
    },

    /// Version revoke (only revoke workflow state, does not undo git commit)
    Revoke(RevokeArgs),

    /// Declare current work associated task/issue
    Claim(ClaimArgs),

    /// Install and register to agent
    Setup(SetupArgs),

    /// Self-update binary and plugins
    Update,

    /// Installation status diagnostics
    SelfCheck,
}

#[derive(clap::Args)]
pub struct SetupArgs {
    /// Target agent (claude/codex/all)
    #[arg(long)]
    pub agent: Option<String>,
}

#[derive(Subcommand)]
pub enum ArchiveCommands {
    /// List all archived versions
    List {
        #[arg(long)]
        branch: Option<String>,
    },
    /// Show archive details for a version
    Show { version: String },
    /// Query archived tasks
    Tasks {
        #[arg(long)]
        version: Option<String>,
        #[arg(long)]
        priority: Option<String>,
    },
    /// Query archived issues
    Issues {
        #[arg(long)]
        version: Option<String>,
        #[arg(long)]
        severity: Option<String>,
    },
    /// Output archived document raw text
    Doc {
        version: String,
        /// Document type (PRD/SPEC/TEST)
        doc_type: String,
    },
    /// Migrate from directory to SQLite
    Migrate {
        /// Delete original directories after migration
        #[arg(long)]
        delete_originals: bool,
    },
    /// Archive statistics
    Stats,
}

#[derive(clap::Args)]
pub struct InitArgs {
    /// Project name
    #[arg(long)]
    pub name: String,

    /// Development mode (full/quick/fast/mvp)
    #[arg(long, default_value = "quick")]
    pub mode: String,
}

#[derive(clap::Args)]
pub struct StatusArgs {
    /// Get specific field only
    #[arg(long)]
    pub field: Option<String>,

    /// Set phase
    #[arg(long)]
    pub phase: Option<String>,

    /// Set mode
    #[arg(long)]
    pub mode: Option<String>,

    /// Set execution mode
    #[arg(long)]
    pub exec_mode: Option<String>,

    /// Set project name
    #[arg(long)]
    pub name: Option<String>,

    /// Set minor version goal
    #[arg(long)]
    pub goals_minor: Option<String>,

    /// Set major version goal
    #[arg(long)]
    pub goals_major: Option<String>,
}

#[derive(clap::Args)]
pub struct IterateArgs {
    /// Archive topic (required, used for archive directory naming)
    #[arg(long)]
    pub topic: String,

    /// commit type (feat/fix/refactor/docs/perf/test/style/workflow), must be explicitly specified
    #[arg(long)]
    pub r#type: String,

    /// List of files/directories to commit (space-separated)
    #[arg(long, num_args = 1..)]
    pub files: Vec<String>,

    /// bump type (default patch, create tag when explicitly specifying minor/major)
    #[arg(short = 'v', long, default_value = "patch")]
    pub bump: String,

    /// Force tag for this patch
    #[arg(long)]
    pub tag: bool,

    /// Confirm execution of last previewed iteration
    #[arg(long)]
    pub confirm: bool,
}

#[derive(clap::Args)]
pub struct DocArgs {
    /// Document type (task/issue/prd/spec/test/brainstorm/changelog/init)
    pub doc_type: String,

    /// Output markdown format document specification
    #[arg(long)]
    pub md: bool,

    /// Output JSON format document specification
    #[arg(long)]
    pub json: bool,

    /// Number of entries
    #[arg(short = 'n', long, default_value = "1")]
    pub count: u32,

    /// issue source
    #[arg(long)]
    pub source: Option<String>,

    /// Project name (for doc init)
    #[arg(long)]
    pub project_name: Option<String>,

    /// git ref starting point (for doc check-sync)
    #[arg(long)]
    pub since: Option<String>,
}

#[derive(clap::Args)]
pub struct DevtestArgs {
    /// Specify task ID
    #[arg(long)]
    pub task: Option<String>,
}

#[derive(clap::Args)]
pub struct TestArgs {
    /// Run specified test file
    #[arg(long)]
    pub file: Option<String>,
}

#[derive(clap::Args)]
pub struct VersionArgs {
    /// Manually set version number
    #[arg(long)]
    pub set: Option<String>,

    /// Bump by type (major/minor/patch)
    #[arg(long)]
    pub bump: Option<String>,
}

#[derive(clap::Args)]
pub struct RevokeArgs {
    /// Target version number to revoke to
    #[arg(long)]
    pub version: Option<String>,

    /// List revokable versions
    #[arg(long)]
    pub list: bool,
}

#[derive(clap::Args)]
pub struct ClaimArgs {
    /// ID to claim (TASK-xxx or ISSUE-xxx), show current status if not provided
    pub ids: Vec<String>,

    /// Release claim (release all if no ID provided)
    #[arg(long)]
    pub revoke: bool,
}

#[derive(Subcommand)]
pub enum InboxCommands {
    /// Generate project context summary (for agent use)
    Context,
}

#[derive(clap::Args)]
pub struct IssueArgs {
    /// List unclosed issues
    #[arg(long)]
    pub list: bool,
}

#[derive(Subcommand)]
pub enum HooksCommands {
    /// Inject context
    Context,

    /// Determine if file write is allowed
    Guard {
        /// File path
        file: Option<String>,
    },

    /// Post-write linkage
    PostWrite {
        /// File path (fallback: TOOL_INPUT_FILE_PATH env var)
        file: Option<String>,
    },

    /// Detect branch switch after Bash execution
    PostBash {
        /// Bash command content (fallback to TOOL_INPUT env var)
        command: Option<String>,
    },

    /// Save CHANGELOG at session end
    SaveChangelog,
}
