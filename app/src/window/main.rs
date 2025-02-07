use std::sync::Arc;

use super::{
    ActiveEventLoop, DevicesManager, Env, Events, EventsManager, WindowHandler, WindowId, RUNTIME,
};

use self::router::MessageRouter;

use anyhow::Result;
use tokio::sync::{mpsc::unbounded_channel, RwLock};
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
                        let (tx, mut rx) = unbounded_channel();
                        let message_router = Arc::new(MessageRouter::new(tx)?);

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
                                message_router: message_router.clone(),
                            },
                        )?;

                        let page_ = Arc::downgrade(&page);
                        RUNTIME.spawn(async move {
                            while let Some(message) = rx.recv().await {
                                if let Some(page) = page_.upgrade() {
                                    page.send_message(&message);
                                } else {
                                    break;
                                }
                            }
                        });

                        let mut watcher = self.devices_manager.get_watcher();
                        RUNTIME.spawn(async move {
                            while watcher.change().await {
                                // message_router.send(message).await;
                            }
                        });

                        // The standalone windows created by cef have many limitations that cannot
                        // be adjusted directly through configuration. Here the windows created by
                        // cef are adjusted directly through the system's window management API.
                        update_page_window_style(&page)?;

                        page.set_devtools_state(true);

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
    message_router: Arc<MessageRouter>,
}

impl Observer for PageObserver {
    fn on_state_change(&self, state: PageState) {
        if state == PageState::Close {
            self.events_manager
                .send(WindowId::Main, Events::DisableWindow);
        }
    }

    fn on_message(&self, message: String) {
        let _ = RUNTIME.block_on(self.message_router.send(message));
    }
}

fn update_page_window_style(page: &Page) -> Result<()> {
    #[cfg(target_os = "windows")]
    use raw_window_handle::{RawWindowHandle, Win32WindowHandle};

    match page.window_handle() {
        #[cfg(target_os = "windows")]
        RawWindowHandle::Win32(Win32WindowHandle { hwnd, .. }) => {
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
        _ => ()
    }

    Ok(())
}

mod router {
    use std::{collections::HashMap, future::Future, sync::atomic::{AtomicU64, Ordering}, time::Duration};

    use anyhow::{anyhow, Result};
    use serde::{de::DeserializeOwned, Deserialize, Serialize};
    use serde_json::{json, Value};
    use tokio::{sync::{mpsc::{unbounded_channel, UnboundedSender}, oneshot::{channel, Sender}, Mutex}, time::timeout};

    use crate::RUNTIME;

    pub(crate) struct MessageRouter {
        sequence: AtomicU64,
        sender: UnboundedSender<String>,
        requests: Mutex<HashMap<u64, Sender<Value>>>,
        responses: Mutex<HashMap<u64, Sender<Value>>>,
    }
    
    impl MessageRouter {
        pub(crate) fn new(sender: UnboundedSender<String>) -> Result<Self> {
            Ok(Self {
                sequence: AtomicU64::new(0),
                requests: Default::default(),
                responses: Default::default(),
                sender,
            })
        }
    
        pub(crate) async fn send(&self, message: String) -> Result<()> {
            let respone: Payload<Value> = serde_json::from_str(&message)?;
            // if let Some(tx) = self.requests.lock().await.remove(&respone.sequence) {
            //     let _ = tx.send(respone.content);
            // }
    
            Ok(())
        }
    
        pub(crate) async fn call<Q, S>(&self, method: &str, content: Q) -> Result<S> 
        where 
            Q: Serialize, 
            S: DeserializeOwned,
        {
            let sequence = self.sequence.fetch_add(1, Ordering::SeqCst);

            self.sender.send(serde_json::to_string(&Payload::Request { method: method.to_string(), sequence, content })?)?;
    
            let (tx, rx) = channel();
            self.requests.lock().await.insert(sequence, tx);
    
            let response = match timeout(Duration::from_secs(5), rx).await {
                Err(_) | Ok(Err(_)) => {
                    drop(self.requests.lock().await.remove(&sequence));
    
                    return Err(anyhow!("request timeout"));
                }
                Ok(Ok(it)) => it,
            };
    
            let response: ResponseContent<S> = serde_json::from_value(response)?;
            response.into()
        }

        pub(crate) async fn on<T, Q, S, F>(&self, method: &str, handler: T) 
        where 
            T: Fn() -> F,
            Q: Serialize, 
            S: DeserializeOwned,
            F: Future<Output = S>,
        {
            let (tx, rx) = unbounded_channel();
            

            RUNTIME.spawn(async move {

            });
        }
    }
    
    #[derive(Deserialize)]
    #[serde(tag = "type", content = "content")]
    enum ResponseContent<T> {
        Ok(T),
        Err(String),
    }
    
    impl<T> Into<Result<T>> for ResponseContent<T> {
        fn into(self) -> Result<T> {
            match self {
                Self::Ok(it) => Ok(it),
                Self::Err(e) => Err(anyhow!("{}", e)),
            }
        }
    }

    #[derive(Deserialize, Serialize)]
    #[serde(tag = "type", content = "content")]
    enum Payload<T> {
        Request {
            method: String,
            sequence: u64,
            content: T,
        },
        Response {
            sequence: u64,
            content: T,
        }
    }
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
