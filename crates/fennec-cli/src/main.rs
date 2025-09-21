use anyhow::Result;
use clap::Parser;
use fennec_core::config::Config;
use fennec_orchestration::SessionManager;
use fennec_security::audit::AuditLogger;
use fennec_tui::app::App;
use tracing::{error, info};
use tracing_subscriber;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Working directory to operate in
    #[arg(short = 'C', long = "cd")]
    working_dir: Option<std::path::PathBuf>,

    /// Sandbox mode
    #[arg(long, value_enum, default_value = "workspace-write")]
    sandbox: SandboxMode,

    /// Ask for approval on actions
    #[arg(long)]
    ask_for_approval: bool,

    /// Configuration file path
    #[arg(long)]
    config: Option<std::path::PathBuf>,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum SandboxMode {
    ReadOnly,
    WorkspaceWrite,
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
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Load environment variables
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    info!("Starting Fennec AI Assistant");

    // Set working directory if specified
    if let Some(working_dir) = &cli.working_dir {
        std::env::set_current_dir(working_dir)?;
        info!("Changed working directory to: {}", working_dir.display());
    }

    // Load configuration
    let config = Config::load(cli.config.as_deref()).await?;

    // Initialize audit logger
    let audit_logger = AuditLogger::new(&config).await?;

    // Initialize session manager
    let session_manager = SessionManager::new(config.clone(), audit_logger).await?;

    // Initialize and run TUI
    let mut app = App::new(session_manager, cli.sandbox.into()).await?;

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
