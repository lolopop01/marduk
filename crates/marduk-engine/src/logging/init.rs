use std::sync::Once;

/// Logger configuration.
///
/// `env_filter` follows the `env_logger` filter syntax (e.g. "info", "warn",
/// "marduk_engine=debug,wgpu=warn").
///
/// `write_style` controls ANSI coloring behavior.
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub env_filter: Option<String>,
    pub write_style: env_logger::WriteStyle,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            env_filter: None,
            write_style: env_logger::WriteStyle::Auto,
        }
    }
}

static INIT: Once = Once::new();

/// Initializes the global logger once.
///
/// This function is idempotent; subsequent calls are ignored.
/// Intended usage is early in `main`.
pub fn init_logging(config: LoggingConfig) {
    INIT.call_once(|| {
        let mut builder = env_logger::Builder::new();

        if let Some(filter) = config.env_filter {
            builder.parse_filters(&filter);
        } else if let Ok(filter) = std::env::var("RUST_LOG") {
            builder.parse_filters(&filter);
        } else {
            // Conservative default. UI engines often prefer info-level visibility.
            builder.filter_level(log::LevelFilter::Info);
        }

        builder.write_style(config.write_style);

        // Timestamping can be refined later (UTC vs local). Default format is fine.
        builder.init();

        log::debug!("logging initialized");
    });
}