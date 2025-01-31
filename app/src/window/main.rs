use std::sync::Arc;

use super::{ActiveEventLoop, Events, WindowHandler, WindowId};
use crate::RUNTIME;

use anyhow::Result;
use webview::{Observer, Page, PageOptions, PageState, Webview};

pub struct MainWindow {
    webview: Arc<Webview>,
    page: Option<Arc<Page>>,
}

impl MainWindow {
    pub fn new(webview: Arc<Webview>) -> Self {
        Self {
            page: None,
            webview,
        }
    }
}

impl WindowHandler for MainWindow {
    fn id(&self) -> WindowId {
        WindowId::Main
    }

    fn user_event(&mut self, _: &ActiveEventLoop, event: &Events) -> Result<()> {
        match event {
            Events::EnableWindow => {
                if self.page.is_none() {
                    self.page.replace(RUNTIME.block_on(self.webview.create_page(
                        "https://google.com",
                        &PageOptions {
                            frame_rate: 30,
                            width: 400,
                            height: 300,
                            device_scale_factor: 1.0,
                            is_offscreen: false,
                            window_handle: None,
                        },
                        PageObserver,
                    ))?);
                }
            }
            Events::DisableWindow => {
                drop(self.page.take());
            }
            _ => (),
        }

        Ok(())
    }
}

struct PageObserver;

impl Observer for PageObserver {
    fn on_state_change(&self, state: PageState) {
        println!("===================== {:?}", state)
    }
}
