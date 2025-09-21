use anyhow::Result;
use clap::Parser;
use fennec_core::config::Config;
use fennec_orchestration::SessionManager;
use fennec_security::audit::AuditLogger;
use fennec_security::{create_sandbox_policy, ApprovalManager};
use fennec_tui::app::App;
use tracing::{error, info, warn};
use tracing_subscriber;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(
    after_help = "SANDBOX LEVELS:\n  read-only         Only file reading, no writes or execution\n  workspace-write   Read/write within workspace, limited execution\n  danger-full-access Full system access (with approval)\n\nSECURITY:\n  Use --ask-for-approval to require explicit consent for potentially dangerous operations.\n  The --cd flag validates and restricts operations to the specified working directory."
)]
struct Cli {
    /// Working directory to operate in
    #[arg(
        short = 'C',
        long = "cd",
        help = "Set working directory and validate it exists"
    )]
    working_dir: Option<std::path::PathBuf>,

    /// Sandbox security level
    #[arg(
        long,
        value_enum,
        default_value = "workspace-write",
        help = "Set sandbox security level"
    )]
    sandbox: SandboxMode,

    /// Require approval for potentially dangerous operations
    #[arg(
        long,
        help = "Ask for user approval before executing potentially dangerous operations"
    )]
    ask_for_approval: bool,

    /// Auto-approve low-risk operations (only with --ask-for-approval)
    #[arg(
        long,
        requires = "ask_for_approval",
        help = "Automatically approve low-risk operations without prompting"
    )]
    auto_approve_low_risk: bool,

    /// Configuration file path
    #[arg(long, help = "Path to configuration file")]
    config: Option<std::path::PathBuf>,

    /// Enable verbose logging
    #[arg(short, long, help = "Enable verbose logging")]
    verbose: bool,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum SandboxMode {
    #[value(name = "read-only", help = "Only file reading, no writes or execution")]
    ReadOnly,
    #[value(
        name = "workspace-write",
        help = "Read/write within workspace, limited execution"
    )]
    WorkspaceWrite,
    #[value(
        name = "danger-full-access",
        help = "Full system access (requires approval)"
    )]
    DangerFullAccess,
}

impl From<SandboxMode> for fennec_security::SandboxLevel {
    fn from(mode: SandboxMode) -> Self {
        match mode {
            SandboxMode::ReadOnly => Self::ReadOnly,
            SandboxMode::WorkspaceWrite => Self::WorkspaceWrite,
            SandboxMode::DangerFullAccess => Self::FullAccess,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables before parsing configuration
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    // Initialize tracing based on verbosity or environment
    let env_filter = if cli.verbose {
        tracing_subscriber::EnvFilter::new("debug")
    } else {
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
    };

    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    info!("Starting Fennec AI Assistant");
    info!("Sandbox level: {:?}", cli.sandbox);
    info!("Approval required: {}", cli.ask_for_approval);

    // Validate and set working directory if specified
    if let Some(working_dir) = &cli.working_dir {
        if !working_dir.exists() {
            error!(
                "Working directory does not exist: {}",
                working_dir.display()
            );
            return Err(anyhow::anyhow!(
                "Working directory does not exist: {}",
                working_dir.display()
            ));
        }

        if !working_dir.is_dir() {
            error!(
                "Working directory is not a directory: {}",
                working_dir.display()
            );
            return Err(anyhow::anyhow!(
                "Working directory is not a directory: {}",
                working_dir.display()
            ));
        }

        // Canonicalize the path to resolve any symlinks or relative components
        let canonical_dir = working_dir.canonicalize().map_err(|e| {
            error!("Failed to canonicalize working directory: {}", e);
            anyhow::anyhow!("Failed to canonicalize working directory: {}", e)
        })?;

        std::env::set_current_dir(&canonical_dir).map_err(|e| {
            error!("Failed to change to working directory: {}", e);
            anyhow::anyhow!("Failed to change to working directory: {}", e)
        })?;

        info!("Changed working directory to: {}", canonical_dir.display());
    }

    // Create sandbox policy
    let sandbox_policy = create_sandbox_policy(
        cli.sandbox.clone().into(),
        cli.working_dir.as_deref(),
        cli.ask_for_approval,
    )
    .map_err(|e| {
        error!("Failed to create sandbox policy: {}", e);
        anyhow::anyhow!("Failed to create sandbox policy: {}", e)
    })?;

    info!(
        "Sandbox policy created - Level: {}, Workspace: {}, Approval: {}",
        sandbox_policy.level(),
        sandbox_policy.workspace_path().display(),
        sandbox_policy.requires_approval()
    );

    // Create approval manager
    let approval_manager = ApprovalManager::new(
        cli.auto_approve_low_risk,
        true, // Always use interactive mode for CLI
    );

    // Load configuration
    let config = Config::load(cli.config.as_deref()).await.map_err(|e| {
        error!("Failed to load configuration: {}", e);
        anyhow::anyhow!("Failed to load configuration: {}", e)
    })?;

    // Initialize audit logger
    let audit_logger = AuditLogger::new(&config).await.map_err(|e| {
        error!("Failed to initialize audit logger: {}", e);
        anyhow::anyhow!("Failed to initialize audit logger: {}", e)
    })?;

    // Initialize session manager with security components
    let session_manager = SessionManager::new(config.clone(), audit_logger)
        .await
        .map_err(|e| {
            error!("Failed to initialize session manager: {}", e);
            anyhow::anyhow!("Failed to initialize session manager: {}", e)
        })?;

    // Display security warning for dangerous sandbox levels
    if matches!(cli.sandbox, SandboxMode::DangerFullAccess) {
        warn!("🔴 WARNING: Running in DANGER-FULL-ACCESS mode!");
        warn!("This mode allows potentially dangerous operations.");
        if !cli.ask_for_approval {
            warn!("⚠️  Consider using --ask-for-approval for additional safety.");
        }
    }

    // Initialize and run TUI with security components
    let mut app = App::new_with_security(session_manager, sandbox_policy, approval_manager)
        .await
        .map_err(|e| {
            error!("Failed to initialize application: {}", e);
            anyhow::anyhow!("Failed to initialize application: {}", e)
        })?;

    match app.run().await {
        Ok(_) => {
            info!("Fennec exited successfully");
            Ok(())
        }
        Err(e) => {
            error!("Fennec encountered an error: {}", e);
            Err(anyhow::anyhow!("Application error: {}", e))
        }
    }
}
