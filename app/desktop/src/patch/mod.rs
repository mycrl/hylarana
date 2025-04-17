// see: https://github.com/rust-windowing/winit/issues/4193
unsafe extern "C" {
    #[link_name = "injectSendEventInWinit"]
    pub fn inject_send_event_in_winit();
}
