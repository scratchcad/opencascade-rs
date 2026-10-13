pub use inner::*;

#[cxx::bridge]
mod inner {
    /// Severity of OpenCASCADE's diagnostic messages.
    #[derive(Debug)]
    #[repr(u32)]
    pub enum Message_Gravity {
        Message_Trace,
        Message_Info,
        Message_Warning,
        Message_Alarm,
        Message_Fail,
    }

    unsafe extern "C++" {
        include!("scratchcad-occt-sys/include/message.hxx");

        type Message_Gravity;

        type Message_ProgressRange;

        #[cxx_name = "construct_unique"]
        pub fn Message_ProgressRange_new() -> UniquePtr<Message_ProgressRange>;

        /// Only messages at least as severe as `level` are printed by the default messenger
        /// (which, among others, prints STEP/IGES transfer statistics to stdout).
        pub fn Message_set_default_trace_level(level: Message_Gravity);

        /// The number of printers attached to the default messenger.
        pub fn Message_default_printer_count() -> i32;
    }
}
