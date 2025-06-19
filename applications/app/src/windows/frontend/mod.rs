mod discovery;
mod service;
mod settings;

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{
        Arc,
        mpsc::{Sender, channel},
    },
    thread,
};

use anyhow::{Result, anyhow};
use discovery::DeviceMetadata;
use hylarana::{
    HylaranaReceiverOptions, HylaranaSenderOptions, MediaStreamDescription, get_runtime_handle,
};

use parking_lot::{Mutex, RwLock};
use raw_window_handle::HasWindowHandle;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use settings::Configure;
use webview::{
    Runtime, RuntimeAttributesBuilder, RuntimeHandler, WebView, WebViewAttributesBuilder,
    WebViewHandler, WebViewState,
};
use winit::{
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowAttributes, WindowId},
};

use self::{discovery::Discovery, service::CoreService, settings::Settings};

use crate::events::{EventChannel, EventTarget, MainEvents, UserEvents};

pub struct Frontend {
    bridge: Arc<Bridge>,
    window: Option<Window>,
    core: Arc<CoreService>,
    runtime: Option<Runtime>,
    webview: Option<Arc<WebView>>,
    events: Arc<EventChannel>,
    transport: Arc<RwLock<Option<Sender<String>>>>,
    remote_window: Arc<RwLock<Option<Arc<Window>>>>,
}

impl Frontend {
    pub fn new(events: Arc<EventChannel>) -> Result<Self> {
        let settings = Settings::new()?;
        let core = CoreService::new()?;
        let discovery = Discovery::new(settings.get().network.bind)?;

        {
            discovery.set_metadata(settings.get().system.name.clone(), Vec::new(), None);
        }

        let transport: Arc<RwLock<Option<Sender<String>>>> = Default::default();
        let bridge = Arc::new(Bridge::new(transport.clone()));
        let settings = Arc::new(Mutex::new(settings));

        bridge.on(
            "GetSettings",
            |settings, _: ()| Ok(settings.lock().get().clone()),
            settings.clone(),
        );

        bridge.on(
            "SetSettings",
            |(discovery, settings), configure: Configure| {
                let mut settings = settings.lock();

                if settings.get().system.name != configure.system.name {
                    discovery.set_metadata(configure.system.name.clone(), Vec::new(), None);
                }

                settings.set(configure)?;
                Ok(())
            },
            (discovery.clone(), settings.clone()),
        );

        bridge.on(
            "GetDevices",
            |manager, _: ()| Ok(manager.get_devices()),
            discovery.clone(),
        );

        bridge.on(
            "GetCaptureSources",
            |_, kind| Ok(CoreService::get_sources(kind)?),
            (),
        );

        bridge.on(
            "CreateSender",
            |(bridge, core, discovery, settings),
             CreateSenderParams {
                 bind,
                 targets,
                 options,
             }: CreateSenderParams| {
                let bridge_ = bridge.clone();
                let settings_ = settings.clone();
                let discovery_ = discovery.clone();
                let (port, description) = core.create_sender(bind, &options, move || {
                    discovery_.set_metadata(
                        settings_.lock().get().system.name.clone(),
                        Vec::new(),
                        None,
                    );

                    let _ = bridge_.send("StatusChangeNotify");
                })?;

                discovery.set_metadata(
                    settings.lock().get().system.name.clone(),
                    targets,
                    Some(DeviceMetadata { port, description }),
                );

                bridge.send("StatusChangeNotify")?;
                Ok(())
            },
            (
                bridge.clone(),
                core.clone(),
                discovery.clone(),
                settings.clone(),
            ),
        );

        bridge.on(
            "CloseSender",
            |(bridge, core), _: ()| {
                core.close_sender();
                bridge.send("StatusChangeNotify")?;

                Ok(())
            },
            (bridge.clone(), core.clone()),
        );

        let remote_window: Arc<RwLock<Option<Arc<Window>>>> = Default::default();
        bridge.on(
            "CreateReceiver",
            |(events, bridge, core, window),
             CreateReceiverParams {
                 addr,
                 options,
                 description,
             }: CreateReceiverParams| {
                let window = if let Some(window) = window.read().clone() {
                    window
                } else {
                    return Err(anyhow!("window not created"));
                };

                events.send(
                    EventTarget::Remote,
                    UserEvents::SetRemoteWindowVisible(true),
                );

                let (tx, rx) = channel();
                {
                    let events_ = events.clone();
                    let bridge_ = bridge.clone();
                    events.run_in_main_thread(move || {
                        let _ = tx.send(core.create_receiver(
                            addr,
                            &options,
                            &description,
                            window,
                            move || {
                                events_.send(
                                    EventTarget::Remote,
                                    UserEvents::SetRemoteWindowVisible(false),
                                );

                                let _ = bridge_.send("StatusChangeNotify");
                            },
                        ));
                    });
                }

                rx.recv()??;
                bridge.send("StatusChangeNotify")?;
                Ok(())
            },
            (
                events.clone(),
                bridge.clone(),
                core.clone(),
                remote_window.clone(),
            ),
        );

        bridge.on(
            "CloseReceiver",
            |(bridge, core), _: ()| {
                core.close_receiver();
                bridge.send("StatusChangeNotify")?;

                Ok(())
            },
            (bridge.clone(), core.clone()),
        );

        bridge.on(
            "GetStatus",
            |core, _: ()| Ok(core.get_status()),
            core.clone(),
        );

        {
            let bridge_ = bridge.clone();
            get_runtime_handle().spawn(async move {
                let mut watcher = discovery.get_watcher().await;

                while watcher.change().await {
                    if bridge_.send("DevicesChangeNotify").is_err() {
                        break;
                    }
                }
            });
        }

        Ok(Self {
            window: None,
            webview: None,
            runtime: None,
            remote_window,
            transport,
            bridge,
            events,
            core,
        })
    }

    pub fn create(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        self.window = Some(
            event_loop.create_window(
                WindowAttributes::default()
                    .with_title("Hylarana")
                    .with_inner_size(PhysicalSize::new(1000, 700))
                    .with_visible(false),
            )?,
        );

        self.runtime = Some(
            RuntimeAttributesBuilder::default()
                .with_browser_subprocess_path(&crate::APP_CONFIG.subprocess_path)
                .with_scheme_dir_path(&crate::APP_CONFIG.cheme_path)
                .with_cache_dir_path(&crate::APP_CONFIG.cache_path)
                .build()
                .create_runtime(RuntimeObserver::new(self.events.clone()))?,
        );

        CoreService::init()?;
        Ok(())
    }

    pub fn window_id(&self) -> Option<WindowId> {
        self.window.as_ref().map(|window| window.id())
    }

    pub fn user_event(&mut self, event: &UserEvents) -> Result<()> {
        match event {
            UserEvents::OnRemoteWindowResized(size) => {
                self.core.resize_receiver(*size);
            }
            UserEvents::OnRemoteWindowView(window) => {
                self.remote_window.write().replace(window.clone());
            }
            UserEvents::OnWebviewAppContextInitialized => {
                if let (Some(runtime), Some(window)) = (&self.runtime, &self.window) {
                    window.set_visible(true);

                    if let Ok(webview) = runtime.create_webview(
                        &crate::APP_CONFIG.uri,
                        &{
                            let size = window.inner_size();
                            WebViewAttributesBuilder::default()
                                .with_window_handle(window.window_handle()?.as_raw())
                                .with_width(size.width)
                                .with_height(size.height)
                                .build()
                        },
                        WebViewObserver::new(self.bridge.clone(), self.events.clone()),
                    ) {
                        let webview = Arc::new(webview);
                        let (tx, rx) = channel::<String>();
                        {
                            let webview_ = webview.clone();
                            thread::spawn(move || {
                                while let Ok(message) = rx.recv() {
                                    webview_.send_message(&message);
                                }
                            });
                        }

                        self.transport.write().replace(tx);
                        self.webview.replace(webview);
                    }
                }
            }
            UserEvents::OnRemoteWindowClose => {
                self.core.close_receiver();
                self.bridge.send("StatusChangeNotify")?;
            }
            #[cfg(target_os = "macos")]
            UserEvents::OnMessagePumpPoll => {
                if self.runtime.is_some() {
                    use webview::RuntimeExtMacos;

                    Runtime::poll();
                }
            }
            _ => (),
        }

        Ok(())
    }

    pub fn window_event(&mut self, event_loop: &ActiveEventLoop, event: &WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => (),
        }
    }
}

struct WebViewObserver {
    bridge: Arc<Bridge>,
    events: Arc<EventChannel>,
}

impl WebViewObserver {
    fn new(bridge: Arc<Bridge>, events: Arc<EventChannel>) -> Self {
        Self { bridge, events }
    }
}

impl WebViewHandler for WebViewObserver {
    fn on_message(&self, message: &str) {
        if let Err(e) = self.bridge.on_message(message.to_string()) {
            log::error!("failed to handle message for webview observer, error={}", e);
        }
    }

    fn on_state_change(&self, state: WebViewState) {
        if state == WebViewState::Close {
            self.events.send_to_main(MainEvents::Shutdown);
        }
    }
}

struct RuntimeObserver(Arc<EventChannel>);

impl RuntimeObserver {
    fn new(events: Arc<EventChannel>) -> Self {
        Self(events)
    }
}

impl RuntimeHandler for RuntimeObserver {
    fn on_context_initialized(&self) {
        self.0.send(
            EventTarget::Frontend,
            UserEvents::OnWebviewAppContextInitialized,
        );
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
        value
            .map(Self::Ok)
            .unwrap_or_else(|e| Self::Err(e.to_string()))
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
        content: ResponseContent<T>,
    },
    Events {
        method: String,
    },
}

struct Bridge {
    table: Arc<RwLock<HashMap<String, Sender<(Sender<Result<Value>>, Value)>>>>,
    transport: Arc<RwLock<Option<Sender<String>>>>,
    tx: Sender<String>,
}

impl Bridge {
    fn new(transport: Arc<RwLock<Option<Sender<String>>>>) -> Self {
        let (tx, rx) = channel::<String>();
        let table: Arc<RwLock<HashMap<String, Sender<(Sender<Result<Value>>, Value)>>>> =
            Default::default();

        {
            let table_ = table.clone();
            let transport_ = transport.clone();
            thread::spawn(move || {
                while let Ok(message) = rx.recv() {
                    match serde_json::from_str(&message) {
                        Ok(Payload::Request {
                            method,
                            sequence,
                            content,
                        }) => {
                            if let Some(sender) = table_.read().get(&method) {
                                let (tx, rx) = channel();

                                if sender.send((tx, content)).is_ok() {
                                    if let Ok(content) = rx.recv() {
                                        log::info!("frontend recv message={:?}", content);

                                        if let Some(tx) = transport_.read().as_ref() {
                                            let _ = tx.send(
                                                serde_json::to_string(&Payload::Response {
                                                    content: ResponseContent::from(content),
                                                    sequence,
                                                })
                                                .unwrap(),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        _ => (),
                    }
                }
            });
        }

        Self {
            transport,
            table,
            tx,
        }
    }

    fn on_message(&self, message: String) -> Result<()> {
        log::info!("frontend send message={}", message);

        self.tx.send(message)?;
        Ok(())
    }

    pub fn send(&self, method: &str) -> Result<()> {
        log::info!("frontend recv event={}", method);

        if let Some(tx) = self.transport.read().as_ref() {
            tx.send(serde_json::to_string(&Payload::<()>::Events {
                method: method.to_string(),
            })?)?;
        }

        Ok(())
    }

    pub fn on<T, Q, S, C>(&self, method: &str, handle: T, ctx: C)
    where
        T: Fn(C, Q) -> Result<S> + Send + 'static,
        Q: DeserializeOwned + Send,
        S: Serialize,
        C: Clone + Send + 'static,
    {
        let (tx, rx) = channel();
        self.table.write().insert(method.to_string(), tx);

        thread::spawn(move || {
            while let Ok((callback, request)) = rx.recv() {
                let func = || {
                    Ok::<_, anyhow::Error>(serde_json::to_value(handle(
                        ctx.clone(),
                        serde_json::from_value(request)?,
                    )?)?)
                };

                let _ = callback.send(func());
            }
        });
    }
}

#[derive(Deserialize)]
struct CreateSenderParams {
    bind: SocketAddr,
    targets: Vec<String>,
    options: HylaranaSenderOptions,
}

#[derive(Deserialize)]
struct CreateReceiverParams {
    addr: SocketAddr,
    options: HylaranaReceiverOptions,
    description: MediaStreamDescription,
}
