// dow/src/
// ├── cli.rs  -- clap subcommand and parameter definitions

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "dow", about = "dev-flow unified CLI dispatcher", long_about = include_str!("../references/binary/dow-help.md"), version = env!("DOW_VERSION"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Human-friendly output (default JSON)
    #[arg(short = 'H', long = "human", global = true)]
    pub human: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Task resource management
    Task {
        #[command(subcommand)]
        command: TaskCommands,
    },

    /// Issue resource management
    Issue {
        #[command(subcommand)]
        command: IssueCommands,
    },

    /// Changelog management
    Changelog {
        #[command(subcommand)]
        command: ChangelogCommands,
    },

    /// Brainstorm document management
    Brainstorm {
        #[command(subcommand)]
        command: BrainstormCommands,
    },

    /// PRD document management
    Prd {
        #[command(subcommand)]
        command: PrdCommands,
    },

    /// SPEC document management
    Spec {
        #[command(subcommand)]
        command: SpecCommands,
    },

    /// Read/write STATUS.yaml
    Status(StatusArgs),

    /// Initialize dev-flow workflow management
    Init(InitArgs),

    /// Diagnose .dev-doc structure, spec, and consistency
    Doctor(DoctorArgs),

    /// Compatibility alias for `dow doctor --fix`
    Fix,

    /// Iteration delivery
    Iterate(IterateArgs),

    /// Project scan
    Scan,

    /// Run test suite
    Test(TestArgs),

    /// Internal common library
    Inbox {
        #[command(subcommand)]
        command: InboxCommands,
    },

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

    /// Version rollback (only rollback workflow state, does not undo git commit)
    Rollback(RollbackArgs),

    /// Declare current work associated task/issue
    Claim(ClaimArgs),

    /// Install and register to agent
    Setup(SetupArgs),

    /// Self-update binary and plugins
    Update,

    /// Installation status diagnostics
    SelfCheck,

    /// Launch web dashboard for project visualization
    Dashboard(DashboardArgs),
}

// ─── Task ────────────────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum TaskCommands {
    /// Create task(s) — accepts flags or stdin JSON
    Create(TaskCreateArgs),
    /// Update task fields by ID
    Update(TaskUpdateArgs),
    /// Remove a pending task (with confirmation and renumbering)
    Remove(TaskRemoveArgs),
    /// List tasks (default: pending only)
    List(TaskListArgs),
    /// Show task details
    Show { id: String },
    /// Mark task(s) as done
    Done { ids: Vec<String> },
    /// Reopen a completed task
    Reopen(TaskReopenArgs),
    /// Output task field schema
    Schema,
}

#[derive(clap::Args)]
pub struct TaskCreateArgs {
    /// Task title
    #[arg(long)]
    pub title: Option<String>,

    /// Task type (feat/fix/refactor/docs/perf/test/style)
    #[arg(long, name = "type")]
    pub task_type: Option<String>,

    /// Priority (P0/P1/P2)
    #[arg(long)]
    pub priority: Option<String>,

    /// Reference (SPEC-AC-xxx or user-request)
    #[arg(long)]
    pub refs: Option<String>,

    /// File scope as a JSON object: {"create":[],"modify":[],"test":[]}
    #[arg(long)]
    pub file: Option<String>,

    /// Dependencies (comma-separated task IDs)
    #[arg(long)]
    pub depends_on: Option<String>,

    /// Can run in parallel
    #[arg(long)]
    pub parallel: bool,

    /// Complexity (S/M/L)
    #[arg(long)]
    pub complexity: Option<String>,

    /// Done-when criteria (comma-separated)
    #[arg(long)]
    pub done_when: Option<String>,
}

#[derive(clap::Args)]
pub struct TaskListArgs {
    /// Show all tasks including completed
    #[arg(long)]
    pub all: bool,
}

#[derive(clap::Args)]
pub struct TaskReopenArgs {
    /// Task ID to reopen
    pub id: String,

    /// Confirmation token (TRO-xxxxxx)
    #[arg(long)]
    pub confirm: Option<String>,
}

#[derive(clap::Args)]
pub struct TaskUpdateArgs {
    /// Task ID to update
    pub id: String,

    /// Task title
    #[arg(long)]
    pub title: Option<String>,

    /// Task type (feat/fix/refactor/docs/perf/test/style)
    #[arg(long, name = "type")]
    pub task_type: Option<String>,

    /// Priority (P0/P1/P2)
    #[arg(long)]
    pub priority: Option<String>,

    /// Reference (SPEC-AC-xxx or user-request)
    #[arg(long)]
    pub refs: Option<String>,

    /// File scope as a JSON object; array items support +item/-item updates
    #[arg(long)]
    pub file: Option<String>,

    /// Dependencies (comma-separated task IDs; +item to append, -item to remove)
    #[arg(long)]
    pub depends_on: Option<String>,

    /// Can run in parallel
    #[arg(long)]
    pub parallel: Option<bool>,

    /// Complexity (S/M/L)
    #[arg(long)]
    pub complexity: Option<String>,

    /// Done-when criteria (comma-separated; +item to append, -item to remove)
    #[arg(long)]
    pub done_when: Option<String>,
}

#[derive(clap::Args)]
pub struct TaskRemoveArgs {
    /// Task ID to remove
    pub id: String,

    /// Confirmation token (TRM-xxxxxx)
    #[arg(long)]
    pub confirm: Option<String>,
}

// ─── Issue ───────────────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum IssueCommands {
    /// Create issue(s) — accepts flags or stdin JSON
    Create(IssueCreateArgs),
    /// Update issue fields by ID
    Update(IssueUpdateArgs),
    /// Remove an open issue (with confirmation and renumbering)
    Remove(IssueRemoveArgs),
    /// List issues (default: open only)
    List(IssueListArgs),
    /// Show issue details
    Show { id: String },
    /// Close issue(s)
    Close { ids: Vec<String> },
    /// Reopen a closed issue
    Reopen(IssueReopenArgs),
    /// Output issue field schema
    Schema,
}

#[derive(clap::Args)]
pub struct IssueCreateArgs {
    /// Issue title
    #[arg(long)]
    pub title: Option<String>,

    /// Severity (P0/P1/P2)
    #[arg(long)]
    pub severity: Option<String>,

    /// Code location
    #[arg(long)]
    pub location: Option<String>,

    /// Description
    #[arg(long)]
    pub desc: Option<String>,

    /// Steps to reproduce
    #[arg(long)]
    pub reproduce: Option<String>,

    /// Issue source (test/audit/other)
    #[arg(long)]
    pub source: Option<String>,

    /// File scope as a JSON object: {"create":[],"modify":[]}
    #[arg(long)]
    pub file: Option<String>,
}

#[derive(clap::Args)]
pub struct IssueListArgs {
    /// Show all issues including closed
    #[arg(long)]
    pub all: bool,
}

#[derive(clap::Args)]
pub struct IssueReopenArgs {
    /// Issue ID to reopen
    pub id: String,

    /// Confirmation token (IRO-xxxxxx)
    #[arg(long)]
    pub confirm: Option<String>,
}

#[derive(clap::Args)]
pub struct IssueUpdateArgs {
    /// Issue ID to update
    pub id: String,

    /// Issue title
    #[arg(long)]
    pub title: Option<String>,

    /// Severity (P0/P1/P2)
    #[arg(long)]
    pub severity: Option<String>,

    /// Code location
    #[arg(long)]
    pub location: Option<String>,

    /// Description
    #[arg(long)]
    pub desc: Option<String>,

    /// Steps to reproduce
    #[arg(long)]
    pub reproduce: Option<String>,

    /// Fix description
    #[arg(long)]
    pub fix: Option<String>,

    /// File scope as a JSON object; array items support +item/-item updates
    #[arg(long)]
    pub file: Option<String>,
}

#[derive(clap::Args)]
pub struct IssueRemoveArgs {
    /// Issue ID to remove
    pub id: String,

    /// Confirmation token (IRM-xxxxxx)
    #[arg(long)]
    pub confirm: Option<String>,
}

// ─── Changelog ───────────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum ChangelogCommands {
    /// List current changelog entries
    List,
    /// Add a changelog entry
    Add(ChangelogAddArgs),
    /// Output changelog field schema
    Schema,
}

#[derive(clap::Args)]
pub struct ChangelogAddArgs {
    /// Entry text
    #[arg(long)]
    pub text: String,
}

// ─── Brainstorm ──────────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum BrainstormCommands {
    /// Create BRAINSTORM.md
    Create,
    /// Output brainstorm document schema
    Schema,
}

// ─── PRD ─────────────────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum PrdCommands {
    /// Create PRD.md
    Create,
    /// Output PRD document schema
    Schema,
}

// ─── SPEC ────────────────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum SpecCommands {
    /// Create SPEC.md
    Create,
    /// Output SPEC document schema
    Schema,
}

// ─── Status ──────────────────────────────────────────────────────────────────

#[derive(clap::Args)]
pub struct StatusArgs {
    #[command(subcommand)]
    pub command: Option<StatusCommands>,

    /// Get specific field only
    #[arg(long)]
    pub field: Option<String>,
}

#[derive(Subcommand)]
pub enum StatusCommands {
    /// Set status fields
    Set(StatusSetArgs),
}

#[derive(clap::Args)]
pub struct StatusSetArgs {
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

// ─── Doctor ──────────────────────────────────────────────────────────────────

#[derive(clap::Args)]
pub struct DoctorArgs {
    /// Auto-fix fixable issues
    #[arg(long)]
    pub fix: bool,
}

// ─── Init ────────────────────────────────────────────────────────────────────

#[derive(clap::Args)]
pub struct InitArgs {
    /// Project name
    #[arg(long)]
    pub name: String,

    /// Development mode (full/quick/fast/mvp)
    #[arg(long, default_value = "quick")]
    pub mode: String,
}

// ─── Iterate ─────────────────────────────────────────────────────────────────

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

    /// Confirmation token (ITR-xxxxxx)
    #[arg(long)]
    pub confirm: Option<String>,
}

// ─── Test ────────────────────────────────────────────────────────────────────

#[derive(clap::Args)]
pub struct TestArgs {
    /// Run tests for a specific Task ID; omit for the full project suite
    #[arg(value_name = "TASK-ID")]
    pub task_id: Option<String>,
}

// ─── Version ─────────────────────────────────────────────────────────────────

#[derive(clap::Args)]
pub struct VersionArgs {
    /// Manually set version number
    #[arg(long)]
    pub set: Option<String>,

    /// Bump by type (major/minor/patch)
    #[arg(long)]
    pub bump: Option<String>,
}

// ─── Rollback ────────────────────────────────────────────────────────────────

#[derive(clap::Args)]
pub struct RollbackArgs {
    /// Target version number to rollback to
    #[arg(long)]
    pub version: Option<String>,

    /// List rollback-able versions
    #[arg(long)]
    pub list: bool,
}

// ─── Claim ───────────────────────────────────────────────────────────────────

#[derive(clap::Args)]
pub struct ClaimArgs {
    /// ID to claim (TASK-xxx or ISSUE-xxx), show current status if not provided
    pub ids: Vec<String>,

    /// Release claim (release all if no ID provided)
    #[arg(long)]
    pub revoke: bool,

    /// Claim duration in seconds (max 1800, default 600)
    #[arg(long)]
    pub timeout: Option<u64>,
}

// ─── Dashboard ──────────────────────────────────────────────────────────────

#[derive(clap::Args)]
pub struct DashboardArgs {
    /// Port to bind (default: auto-select from 9800-9900)
    #[arg(long)]
    pub port: Option<u16>,

    /// Do not open browser automatically
    #[arg(long)]
    pub no_open: bool,
}

// ─── Setup ───────────────────────────────────────────────────────────────────

#[derive(clap::Args)]
pub struct SetupArgs {
    /// Target agent (claude/codex/all)
    #[arg(long)]
    pub agent: Option<String>,
}

// ─── Archive ─────────────────────────────────────────────────────────────────

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

// ─── Inbox ───────────────────────────────────────────────────────────────────

#[derive(Subcommand)]
pub enum InboxCommands {
    /// Generate project context summary (for agent use)
    Context,
}

// ─── Hooks ───────────────────────────────────────────────────────────────────

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

    /// Unified session end handler (revoke claims + save changelog)
    SessionStop,
}
