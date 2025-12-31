use clap::Parser;

use photo_date_bench::{error::AppError, App};
use tracing_subscriber::{
    fmt::format::FmtSpan, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt,
};

fn main() -> Result<(), AppError> {
    #[cfg(target_os = "windows")]
    let events = tracing_subscriber::fmt::layer()
        .with_span_events(FmtSpan::CLOSE)
        .with_ansi(false);
    #[cfg(not(target_os = "windows"))]
    let events = tracing_subscriber::fmt::layer().with_span_events(FmtSpan::CLOSE);
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(events)
        .init();
    let app = App::parse();

    photo_date_bench::run_image_processing(app)
}
