use std::io;

use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::format::FmtSpan;

pub fn init_logging(log_level: &str, log_format: &str, json_output: bool) {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    let builder = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_target(true)
        .with_thread_ids(true);

    if json_output || log_format == "json" {
        builder
            .json()
            .with_current_span(true)
            .with_span_list(true)
            .init();
    } else {
        builder
            .with_ansi(io::IsTerminal::is_terminal(&io::stdout()))
            .init();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_logging_default() {
        init_logging("info", "json", true);
        tracing::info!("test log message");
    }
}
