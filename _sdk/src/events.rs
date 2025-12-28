//! Event emission for smart contracts (memory pool optimized)

use crate::ffi;
use serde::Serialize;

/// Emit an event that can be indexed by off-chain services (optimized - direct serialization)
pub fn emit<T: Serialize>(topic: &str, data: &T) {
    // Use postcard for efficient binary serialization
    if let Ok(data_bytes) = postcard::to_allocvec(data) {
        ffi::emit_event_internal(topic, &data_bytes);
    }
}

/// Log a debug message (only visible in development)
pub fn log(message: &str) {
    ffi::log_message(message);
}

/// Helper macro for creating structured events (memory pool optimized)
#[macro_export]
macro_rules! event {
    ($topic:expr, $($field:ident: $value:expr),* $(,)?) => {
        {
            use $crate::events::emit;
            use serde::Serialize;

            #[derive(Serialize)]
            struct EventData {
                $($field: String),*
            }

            // Create event data with string values
            let event_data = EventData {
                $($field: {
                    alloc::format!("{}", $value)
                }),*
            };

            emit($topic, &event_data);
        }
    };
}
