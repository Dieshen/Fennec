use fennec_core::Result;
use fennec_orchestration::SessionManager;
use fennec_security::SandboxLevel;

pub struct App {
    session_manager: SessionManager,
    sandbox_level: SandboxLevel,
}

impl App {
    pub async fn new(session_manager: SessionManager, sandbox_level: SandboxLevel) -> Result<Self> {
        Ok(Self {
            session_manager,
            sandbox_level,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        // TUI implementation will go here
        Ok(())
    }
}