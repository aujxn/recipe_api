use tracing::Level;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

/// Initialize event tracing
pub fn init() {
    let filter =
        EnvFilter::from_default_env().add_directive(env!("CARGO_PKG_NAME").parse().unwrap());

    tracing_subscriber::fmt()
        //.without_time()
        .with_max_level(Level::TRACE)
        //.with_target(false)
        .with_env_filter(filter)
        .with_writer(std::io::stdout)
        .init();
}
