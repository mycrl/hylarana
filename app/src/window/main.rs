use std::sync::Arc;

use crate::devices::DeviceInfo;

use super::{
    ActiveEventLoop, DevicesManager, Env, Events, EventsManager, WindowHandler, WindowId, RUNTIME,
};

use anyhow::Result;
use common::MediaStreamDescription;
use raw_window_handle::{RawWindowHandle, Win32WindowHandle};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use webview::{Observer, Page, PageOptions, PageState, Webview};

pub struct MainWindow {
    devices_manager: Arc<DevicesManager>,
    events_manager: EventsManager,
    webview: Arc<Webview>,
    page: Option<Arc<Page>>,
    env: Arc<RwLock<Env>>,
}

impl MainWindow {
    const WIDTH: u32 = 1000;
    const HEIGHT: u32 = 600;

    pub fn new(
        env: Arc<RwLock<Env>>,
        devices_manager: Arc<DevicesManager>,
        events_manager: EventsManager,
        webview: Arc<Webview>,
    ) -> Self {
        Self {
            page: None,
            devices_manager,
            events_manager,
            webview,
            env,
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
                        let page = self.webview.create_page(
                            "http://localhost:5173",
                            &PageOptions {
                                frame_rate: 30,
                                width: Self::WIDTH,
                                height: Self::HEIGHT,
                                is_offscreen: false,
                                window_handle: None,
                                device_scale_factor: 1.0,
                            },
                            PageObserver {
                                events_manager: self.events_manager.clone(),
                            },
                        )?;

                        // The standalone windows created by cef have many limitations that cannot
                        // be adjusted directly through configuration. Here the windows created by
                        // cef are adjusted directly through the system's window management API.
                        update_page_window_style(&page)?;

                        page.set_devtools_state(true);
                        // page.on_bridge(PageHandler {
                        //     devices_manager: self.devices_manager.clone(),
                        //     env: self.env.clone(),
                        // });

                        // let page_ = Arc::downgrade(&page);
                        // let mut watcher = self.devices_manager.get_watcher();
                        // RUNTIME.spawn(async move {
                        //     while watcher.change().await {
                        //         if let Some(page) = page_.upgrade() {
                        //             let _ = page
                        //                 .call_bridge::<_, ()>(&MainRequest::DevicesChange)
                        //                 .await;
                        //         }
                        //     }
                        // });

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

// #[derive(Debug, Serialize)]
// #[serde(tag = "type", content = "content")]
// enum MainRequest {
//     DevicesChange,
// }

// #[derive(Debug, Deserialize)]
// #[serde(tag = "type", content = "content")]
// enum PageRequest {
//     GetName,
//     SetName {
//         name: String,
//     },
//     GetDevices,
//     SendDescription {
//         names: Vec<String>,
//         description: MediaStreamDescription,
//     },
//     SetAutoAllow {
//         enable: bool,
//     },
// }

// #[derive(Debug, Serialize)]
// #[serde(tag = "type", content = "content")]
// enum PageRespone {
//     GetName { name: String },
//     SetName,
//     GetDevices { devices: Vec<DeviceInfo> },
//     SendDescription,
//     SetAutoAllow,
// }

// struct PageHandler {
//     devices_manager: Arc<DevicesManager>,
//     env: Arc<RwLock<Env>>,
// }

// #[async_trait]
// impl BridgeObserver for PageHandler {
//     type Req = PageRequest;
//     type Res = PageRespone;
//     type Err = anyhow::Error;

//     async fn on(&self, req: Self::Req) -> Result<Self::Res, Self::Err> {
//         log::info!("main page receiver a request={:?}", req);

//         Ok(match req {
//             PageRequest::GetName => PageRespone::GetName {
//                 name: self.env.read().await.settings.name.clone(),
//             },
//             PageRequest::SetName { name } => {
//                 self.env.write().await.update_name(name)?;

//                 PageRespone::SetName
//             }
//             PageRequest::GetDevices => PageRespone::GetDevices {
//                 devices: self.devices_manager.get_devices().await,
//             },
//             PageRequest::SendDescription { names, description } => {
//                 self.devices_manager
//                     .send_description(names, description)
//                     .await;

//                 PageRespone::SendDescription
//             }
//             PageRequest::SetAutoAllow { enable } => PageRespone::SetAutoAllow,
//         })
//     }
// }
