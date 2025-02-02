use std::sync::Arc;

use super::{ActiveEventLoop, Events, EventsManager, WindowHandler, WindowId};
use crate::RUNTIME;

use anyhow::Result;
use async_trait::async_trait;
use raw_window_handle::{RawWindowHandle, Win32WindowHandle};
use serde::{Deserialize, Serialize};
use webview::{Observer, Page, PageOptions, PageState, Webview, BridgeObserver};

pub struct MainWindow {
    events_manager: EventsManager,
    webview: Arc<Webview>,
    page: Option<Arc<Page>>,
}

impl MainWindow {
    const WIDTH: u32 = 1000;
    const HEIGHT: u32 = 600;

    pub fn new(events_manager: EventsManager, webview: Arc<Webview>) -> Self {
        Self {
            page: None,
            events_manager,
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
                    {
                        let page = RUNTIME.block_on(self.webview.create_page(
                            "webview://index.html",
                            &PageOptions {
                                frame_rate: 30,
                                width: Self::WIDTH,
                                height: Self::HEIGHT,
                                device_scale_factor: 1.0,
                                is_offscreen: false,
                                window_handle: None,
                            },
                            PageObserver {
                                events_manager: self.events_manager.clone(),
                            },
                        ))?;

                        // The standalone windows created by cef have many limitations that cannot
                        // be adjusted directly through configuration. Here the windows created by
                        // cef are adjusted directly through the system's window management API.
                        update_page_window_style(&page)?;

                        page.set_devtools_state(true);
                        page.on_bridge(PageHandler);
                        self.page.replace(page);
                    }
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

struct PageObserver {
    events_manager: EventsManager,
}

impl Observer for PageObserver {
    fn on_state_change(&self, state: PageState) {
        if state == PageState::Close {
            self.events_manager
                .send(WindowId::Main, Events::DisableWindow);
        }
    }
}

fn update_page_window_style(page: &Page) -> Result<()> {
    if let RawWindowHandle::Win32(Win32WindowHandle { hwnd, .. }) = page.window_handle() {
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::{
                Foundation::{HWND, RECT},
                UI::WindowsAndMessaging::{
                    AdjustWindowRectEx, GetWindowLongA, SetWindowLongA, SetWindowPos, GWL_STYLE,
                    SWP_NOMOVE, SWP_NOZORDER, WINDOW_EX_STYLE, WS_MAXIMIZEBOX, WS_OVERLAPPEDWINDOW,
                },
            };

            let mut rect = RECT::default();
            rect.right = MainWindow::WIDTH as i32;
            rect.bottom = MainWindow::HEIGHT as i32;

            unsafe {
                AdjustWindowRectEx(&mut rect, WS_OVERLAPPEDWINDOW, false, WINDOW_EX_STYLE(0))?;
            }

            let hwnd = HWND(hwnd.get() as _);
            let mut style = unsafe { GetWindowLongA(hwnd, GWL_STYLE) };
            style &= !WS_MAXIMIZEBOX.0 as i32;

            unsafe {
                SetWindowLongA(hwnd, GWL_STYLE, style);
            }

            unsafe {
                SetWindowPos(
                    hwnd,
                    None,
                    0,
                    0,
                    rect.right - rect.left,
                    rect.bottom - rect.top,
                    SWP_NOMOVE | SWP_NOZORDER,
                )?;
            }
        }
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
enum Request {
}

#[derive(Debug, Serialize)]
enum Response {

}

struct PageHandler;

#[async_trait]
impl BridgeObserver for PageHandler {
    type Req = Request;
    type Res = Option<Response>;
    type Err = anyhow::Error;

    async fn on(&self, req: Self::Req) -> Result<Self::Res, Self::Err> {
        match req {

        }

        todo!()
    }
}
