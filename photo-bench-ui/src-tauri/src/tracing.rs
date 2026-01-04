use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tracing::{Event, Subscriber};
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

// Define the structure for the log message sent to the UI
#[derive(Clone, Serialize)]
struct LogEntry {
    level: String,
    message: String,
    target: String,
}

// A custom tracing layer to emit logs to the UI
struct TauriLogLayer {
    app_handle: AppHandle,
}

impl<S> Layer<S> for TauriLogLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let metadata = event.metadata();
        let level = metadata.level().to_string();
        let target = metadata.target().to_string();

        // https://github.com/tauri-apps/tauri/issues/8494
        if target == "tao::platform_impl::platform::event_loop::runner" {
            return;
        }

        let mut visitor = JsonVisitor(String::new());
        event.record(&mut visitor);
        let message = visitor.0;

        let log_entry = LogEntry {
            level,
            message,
            target,
        };

        // Emit the event to the frontend
        if let Err(e) = self.app_handle.emit("rust-log", log_entry) {
            eprintln!("Failed to emit log event: {:?}", e);
        }
    }
}

// Helper to format tracing event fields into a single message string
struct JsonVisitor(String);

impl tracing::field::Visit for JsonVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0 = format!("{:?}", value);
        } else {
            // Optional: handle other structured data if needed
            // self.0 = format!("{}: {:?}", field.name(), value);
        }
    }
}

// Initialize tracing and the custom layer
pub fn init_tracing(app_handle: AppHandle) {
    let tauri_layer = TauriLogLayer { app_handle };

    tracing_subscriber::registry()
        .with(tauri_layer)
        // Add other layers if desired (e.g., console output)
        // .with(tracing_subscriber::fmt::Layer::default())
        .init();
}
