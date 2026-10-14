//! Where OpenCASCADE's own diagnostic messages go.
//!
//! By default OpenCASCADE prints them (STEP and IGES transfer statistics, warnings, failures) to
//! stdout. A program whose stdout carries something else (a protocol, structured logs) should
//! call [`set_message_handler`] to receive them itself, or [`set_occt_messages`] to send them to
//! stderr. These settings are process-wide.
//!
//! OpenCASCADE writes a few messages straight to `std::cout` without going through its
//! messenger; those cannot be redirected from here.

use scratchcad_occt_sys::message::{self as ffi, Message_Gravity};
use std::sync::Arc;

/// The severity of an OpenCASCADE message, from least to most severe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MessageLevel {
    Trace,
    Info,
    Warning,
    Alarm,
    Fail,
}

impl MessageLevel {
    fn to_ffi(self) -> Message_Gravity {
        match self {
            MessageLevel::Trace => Message_Gravity::Message_Trace,
            MessageLevel::Info => Message_Gravity::Message_Info,
            MessageLevel::Warning => Message_Gravity::Message_Warning,
            MessageLevel::Alarm => Message_Gravity::Message_Alarm,
            MessageLevel::Fail => Message_Gravity::Message_Fail,
        }
    }

    fn from_ffi(gravity: Message_Gravity) -> Self {
        match gravity {
            Message_Gravity::Message_Trace => MessageLevel::Trace,
            Message_Gravity::Message_Info => MessageLevel::Info,
            Message_Gravity::Message_Warning => MessageLevel::Warning,
            Message_Gravity::Message_Alarm => MessageLevel::Alarm,
            _ => MessageLevel::Fail,
        }
    }
}

/// Sends every OpenCASCADE message at or above `level` to `handler` instead of stdout.
///
/// The handler may be called from any thread, including OpenCASCADE's worker threads; a
/// handler that panics loses that message.
pub fn set_message_handler(
    level: MessageLevel,
    handler: impl Fn(MessageLevel, &str) + Send + Sync + 'static,
) {
    ffi::set_message_handler(Some(Arc::new(move |gravity: Message_Gravity, text: &str| {
        handler(MessageLevel::from_ffi(gravity), text)
    })));
    ffi::Message_use_rust_printer(level.to_ffi());
}

/// Shows OpenCASCADE's messages (from [`MessageLevel::Info`] up) on stdout, as it does by
/// default, or, when `enabled` is false, only its failures, on stderr.
pub fn set_occt_messages(enabled: bool) {
    ffi::set_message_handler(None);
    if enabled {
        ffi::Message_use_stream_printer(false, Message_Gravity::Message_Info);
    } else {
        ffi::Message_use_stream_printer(true, Message_Gravity::Message_Fail);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_round_trip_and_are_ordered() {
        let levels = [
            MessageLevel::Trace,
            MessageLevel::Info,
            MessageLevel::Warning,
            MessageLevel::Alarm,
            MessageLevel::Fail,
        ];
        for level in levels {
            assert_eq!(MessageLevel::from_ffi(level.to_ffi()), level);
        }
        assert!(levels.windows(2).all(|pair| pair[0] < pair[1]));
    }
}
