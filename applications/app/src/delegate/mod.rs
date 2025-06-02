// see: https://github.com/rust-windowing/winit/issues/4193
unsafe extern "C" {
    #[link_name = "injectDelegate"]
    pub fn inject_delegate();
}
