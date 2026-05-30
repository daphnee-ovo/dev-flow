// dow/src/
// ├── cli.rs  -- clap 子命令与参数定义

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "dow", about = "dev-flow 统一 CLI 调度器", version = env!("DOW_VERSION"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// 人类友好输出（默认 JSON）
    #[arg(short = 'H', long = "human", global = true)]
    pub human: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 读写 STATUS.yaml
    Status(StatusArgs),

    /// 初始化 dev-flow 工作流管理
    Init(InitArgs),

    /// 文档规范检查
    Check,

    /// 迭代交付
    Iterate(IterateArgs),

    /// 项目扫描
    Scan,

    /// 校验 .dev-doc 结构
    Validate,

    /// 修复 .dev-doc 文件格式问题
    Fix,

    /// 生成文档模板
    Doc(DocArgs),

    /// 任务级测试
    Devtest(DevtestArgs),

    /// 运行全量测试
    Test(TestArgs),

    /// 内部公用库
    Inbox {
        #[command(subcommand)]
        command: InboxCommands,
    },

    /// Issue 管理
    Issue(IssueArgs),

    /// 读写 VERSION
    Version(VersionArgs),

    /// 归档查询
    Archive {
        #[command(subcommand)]
        command: ArchiveCommands,
    },

    /// Hook 子命令
    Hooks {
        #[command(subcommand)]
        command: HooksCommands,
    },

    /// 安装并注册到 agent
    Setup(SetupArgs),

    /// 自更新二进制和插件
    Update,

    /// 安装状态诊断
    SelfCheck,
}

#[derive(clap::Args)]
pub struct SetupArgs {
    /// 目标 agent（claude/codex/all）
    #[arg(long)]
    pub agent: Option<String>,
}

#[derive(Subcommand)]
pub enum ArchiveCommands {
    /// 列出所有归档版本
    List {
        #[arg(long)]
        branch: Option<String>,
    },
    /// 显示某版本归档详情
    Show {
        version: String,
    },
    /// 查询归档任务
    Tasks {
        #[arg(long)]
        version: Option<String>,
        #[arg(long)]
        priority: Option<String>,
    },
    /// 查询归档 issue
    Issues {
        #[arg(long)]
        version: Option<String>,
        #[arg(long)]
        severity: Option<String>,
    },
    /// 输出归档文档原文
    Doc {
        version: String,
        /// 文档类型（PRD/SPEC/TEST）
        doc_type: String,
    },
    /// 从目录迁移到 SQLite
    Migrate {
        /// 迁移后删除原始目录
        #[arg(long)]
        delete_originals: bool,
    },
    /// 归档统计
    Stats,
}

#[derive(clap::Args)]
pub struct InitArgs {
    /// 项目名称
    #[arg(long)]
    pub name: String,

    /// 开发模式（full/quick/fast/mvp）
    #[arg(long, default_value = "quick")]
    pub mode: String,
}

#[derive(clap::Args)]
pub struct StatusArgs {
    /// 只获取某字段
    #[arg(long)]
    pub field: Option<String>,

    /// 设置阶段
    #[arg(long)]
    pub phase: Option<String>,

    /// 设置模式
    #[arg(long)]
    pub mode: Option<String>,

    /// 设置执行模式
    #[arg(long)]
    pub exec_mode: Option<String>,

    /// 设置项目名
    #[arg(long)]
    pub name: Option<String>,

    /// 设置 minor 版本目标
    #[arg(long)]
    pub goals_minor: Option<String>,

    /// 设置 major 版本目标
    #[arg(long)]
    pub goals_major: Option<String>,
}

#[derive(clap::Args)]
pub struct IterateArgs {
    /// 归档主题（必填，用于归档目录命名）
    #[arg(long)]
    pub topic: String,

    /// commit 类型（feat/fix/refactor/docs/perf/test/style/workflow），必须显式指定
    #[arg(long)]
    pub r#type: String,

    /// 要提交的文件/目录列表（空格分隔）
    #[arg(long, num_args = 1..)]
    pub files: Vec<String>,

    /// bump 类型（默认 patch，显式指定 minor/major 时打 tag）
    #[arg(short = 'v', long, default_value = "patch")]
    pub bump: String,

    /// 强制对本次 patch 打 tag
    #[arg(long)]
    pub tag: bool,

    /// 确认执行上次预览的迭代
    #[arg(long)]
    pub confirm: bool,
}

#[derive(clap::Args)]
pub struct DocArgs {
    /// 文档类型（task/issue/prd/spec/test/brainstorm/changelog）
    pub doc_type: String,

    /// 输出 markdown 格式的文档规范
    #[arg(long)]
    pub md: bool,

    /// 输出 JSON 格式的文档规范
    #[arg(long)]
    pub json: bool,

    /// 条目数量
    #[arg(short = 'n', long, default_value = "1")]
    pub count: u32,

    /// issue 来源
    #[arg(long)]
    pub source: Option<String>,
}

#[derive(clap::Args)]
pub struct DevtestArgs {
    /// 指定任务 ID
    #[arg(long)]
    pub task: Option<String>,
}

#[derive(clap::Args)]
pub struct TestArgs {
    /// 运行指定测试文件
    #[arg(long)]
    pub file: Option<String>,
}

#[derive(clap::Args)]
pub struct VersionArgs {
    /// 手动设定版本号
    #[arg(long)]
    pub set: Option<String>,

    /// 按类型 bump（major/minor/patch）
    #[arg(long)]
    pub bump: Option<String>,
}

#[derive(Subcommand)]
pub enum InboxCommands {
    /// 生成项目上下文摘要（供 agent 使用）
    Context,
}

#[derive(clap::Args)]
pub struct IssueArgs {
    /// 列出未关闭的 issue
    #[arg(long)]
    pub list: bool,
}

#[derive(Subcommand)]
pub enum HooksCommands {
    /// 注入上下文
    Context,

    /// 判断文件是否允许写入
    Guard {
        /// 文件路径
        file: Option<String>,
    },

    /// 文件写入后联动
    PostWrite {
        /// 文件路径（fallback 读 TOOL_INPUT_FILE_PATH 环境变量）
        file: Option<String>,
    },

    /// Bash 执行后检测分支切换
    PostBash {
        /// Bash 命令内容（fallback 读 TOOL_INPUT 环境变量）
        command: Option<String>,
    },

    /// 会话结束保存 CHANGELOG
    SaveChangelog,
}
