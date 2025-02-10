use std::sync::Arc;

use super::{
    ActiveEventLoop, DevicesManager, Env, Events, EventsManager, WindowHandler, WindowId, RUNTIME,
};

use self::router::MessageRouter;

use anyhow::Result;
use hylarana::Capture;
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
            Events::CreateWindow => {
                if self.page.is_none() {
                    {
                        let (tx, mut rx) = unbounded_channel();
                        let message_router = Arc::new(MessageRouter::new(tx)?);

                        {
                            message_router.on(
                                "GetName",
                                |env, _: ()| async move {
                                    Ok(env.read().await.settings.name.clone())
                                },
                                self.env.clone(),
                            );

                            message_router.on(
                                "SetName",
                                |env, name: String| async move {
                                    env.write().await.update_name(name)?;
                                    Ok(())
                                },
                                self.env.clone(),
                            );

                            message_router.on(
                                "GetDevices",
                                |devices_manager, _: ()| async move {
                                    Ok(devices_manager.get_devices().await)
                                },
                                self.devices_manager.clone(),
                            );

                            message_router.on(
                                "GetCaptureSources",
                                |_, kind| async move {
                                    Ok(RUNTIME
                                        .spawn_blocking(move || Capture::get_sources(kind))
                                        .await??)
                                },
                                (),
                            );
                        }

                        let page = self.webview.create_page(
                            {
                                &std::env::var(Env::ENV_WEBVIEW_MAIN_PAGE_URL)
                                    .unwrap_or_else(|_| "webview://index.html".to_string())
                            },
                            &PageOptions {
                                width: Self::WIDTH,
                                height: Self::HEIGHT,
                                ..Default::default()
                            },
                            PageObserver {
                                events_manager: self.events_manager.clone(),
                                message_router: message_router.clone(),
                            },
                        )?;

                        // The standalone windows created by cef have many limitations that cannot
                        // be adjusted directly through configuration. Here the windows created by
                        // cef are adjusted directly through the system's window management API.
                        update_page_window_style(&page)?;

                        {
                            let page_ = Arc::downgrade(&page);
                            RUNTIME.spawn(async move {
                                while let Some(message) = rx.recv().await {
                                    if let Some(page) = page_.upgrade() {
                                        page.send_message(&message);

                                        log::info!("app message router send message={}", message);
                                    } else {
                                        break;
                                    }
                                }
                            });

                            let message_router_ = Arc::downgrade(&message_router);
                            let mut watcher = self.devices_manager.get_watcher();
                            RUNTIME.spawn(async move {
                                while watcher.change().await {
                                    if let Some(message_router) = message_router_.upgrade() {
                                        let _ = message_router
                                            .call::<_, ()>("DevicesChangeNotify", ())
                                            .await;
                                    } else {
                                        break;
                                    }
                                }
                            });
                        }

                        if std::env::var(Env::ENV_ENABLE_WEBVIEW_DEVTOOLS).is_ok() {
                            page.set_devtools_state(true);
                        }

                        self.page.replace(page);
                        let _ = RUNTIME.block_on(message_router.call::<_, ()>("ReadyNotify", ()));
                    }
                }
            }
            Events::CloseWindow => {
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
                .send(WindowId::Main, Events::CloseWindow);
        }
    }

    fn on_message(&self, message: String) {
        if let Err(e) = RUNTIME.block_on(self.message_router.send(message)) {
            log::warn!("failed to send message to message router, error={:?}", e);
        }
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
        _ => (),
    }

    Ok(())
}

mod router {
    use std::{
        collections::HashMap,
        future::Future,
        sync::atomic::{AtomicU64, Ordering},
        time::Duration,
    };

    use anyhow::{anyhow, Result};
    use serde::{de::DeserializeOwned, Deserialize, Serialize};
    use serde_json::Value;
    use tokio::{
        sync::{
            mpsc::{unbounded_channel, UnboundedSender},
            oneshot::{channel, Sender},
            Mutex, RwLock,
        },
        time::timeout,
    };

    pub(crate) struct MessageRouter {
        sequence: AtomicU64,
        message_channel: UnboundedSender<String>,
        // request sender table
        rst: Mutex<HashMap<u64, Sender<Value>>>,
        // on receiver table
        ort: RwLock<HashMap<String, UnboundedSender<(Sender<Result<Value>>, Value)>>>,
    }

    impl MessageRouter {
        pub(crate) fn new(message_channel: UnboundedSender<String>) -> Result<Self> {
            Ok(Self {
                ort: RwLock::new(HashMap::with_capacity(100)),
                rst: Mutex::new(HashMap::with_capacity(100)),
                sequence: AtomicU64::new(0),
                message_channel,
            })
        }

        pub(crate) async fn send(&self, message: String) -> Result<()> {
            log::info!("app message router recv message={}", message);

            let respone: Payload<Value> = serde_json::from_str(&message)?;
            match respone {
                Payload::Request {
                    method,
                    sequence,
                    content,
                } => {
                    if let Some(sender) = self.ort.read().await.get(&method) {
                        let (tx, rx) = channel();
                        sender.send((tx, content))?;

                        self.message_channel
                            .send(serde_json::to_string(&Payload::Response {
                                content: ResponseContent::from(rx.await?),
                                sequence,
                            })?)?;
                    }
                }
                Payload::Response { sequence, content } => {
                    if let Some(tx) = self.rst.lock().await.remove(&sequence) {
                        let _ = tx.send(content);
                    }
                }
            }

            Ok(())
        }

        pub(crate) async fn call<Q, S>(&self, method: &str, content: Q) -> Result<S>
        where
            Q: Serialize,
            S: DeserializeOwned,
        {
            let sequence = self.sequence.fetch_add(1, Ordering::SeqCst);
            self.message_channel
                .send(serde_json::to_string(&Payload::Request {
                    method: method.to_string(),
                    sequence,
                    content,
                })?)?;

            let (tx, rx) = channel();
            self.rst.lock().await.insert(sequence, tx);

            let response = match timeout(Duration::from_secs(5), rx).await {
                Err(_) | Ok(Err(_)) => {
                    drop(self.rst.lock().await.remove(&sequence));

                    return Err(anyhow!("request timeout"));
                }
                Ok(Ok(it)) => it,
            };

            let response: ResponseContent<S> = serde_json::from_value(response)?;
            response.into()
        }

        pub(crate) fn on<T, Q, S, F, C>(&self, method: &str, handle: T, ctx: C)
        where
            T: Fn(C, Q) -> F + Send + Sync + 'static,
            Q: DeserializeOwned + Send,
            S: Serialize,
            F: Future<Output = Result<S>> + Send,
            C: Clone + Sync + Send + 'static,
        {
            let (tx, mut rx) = unbounded_channel();
            self.ort.blocking_write().insert(method.to_string(), tx);

            crate::RUNTIME.spawn(async move {
                while let Some((callback, request)) = rx.recv().await {
                    let func = async {
                        Ok::<_, anyhow::Error>(serde_json::to_value(
                            handle(ctx.clone(), serde_json::from_value(request)?).await?,
                        )?)
                    };

                    let _ = callback.send(func.await);
                }
            });
        }
    }

    #[derive(Deserialize, Serialize)]
    #[serde(tag = "ty", content = "content")]
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

    impl<T> From<Result<T>> for ResponseContent<T> {
        fn from(value: Result<T>) -> Self {
            match value {
                Ok(it) => Self::Ok(it),
                Err(e) => Self::Err(e.to_string()),
            }
        }
    }

    #[derive(Deserialize, Serialize)]
    #[serde(tag = "ty", content = "content")]
    enum Payload<T> {
        Request {
            method: String,
            sequence: u64,
            content: T,
        },
        Response {
            sequence: u64,
            content: T,
        },
    }
}
