mod discovery;
mod service;
mod settings;

use std::{
    collections::HashMap,
    sync::{
        Arc,
        mpsc::{Sender, channel},
    },
    thread,
    time::Duration,
};

use anyhow::{Result, anyhow};
use hylarana::{
    HylaranaReceiverOptions, HylaranaSenderOptions, MediaStreamDescription, VideoDecoderType,
    get_runtime_handle,
};

use parking_lot::{Mutex, RwLock};
use raw_window_handle::HasWindowHandle;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use settings::Configure;
use wew::{
    MessageLoopAbstract, MessagePumpLoop, NativeWindowWebView,
    request::{CustomRequestHandlerFactory, CustomSchemeAttributes, RequestHandlerWithLocalDisk},
    runtime::{LogLevel, MessagePumpRuntimeHandler, Runtime, RuntimeHandler},
    webview::{WebView, WebViewAttributes, WebViewHandler, WebViewState},
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
    runtime: Option<Runtime<MessagePumpLoop, NativeWindowWebView>>,
    page: Option<Arc<WebView<NativeWindowWebView>>>,
    events: Arc<EventChannel>,
    transport: Arc<RwLock<Option<Sender<String>>>>,
    remote_window: Arc<RwLock<Option<Arc<Window>>>>,
}

impl Frontend {
    pub fn new(events: Arc<EventChannel>) -> Result<Self> {
        let settings = Settings::new()?;
        let core = CoreService::new()?;
        let discovery = Discovery::new(settings.get().system.name.clone())?;
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
                    discovery.set_name(configure.system.name.clone());
                    discovery.send(Vec::new(), None);
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
            |(bridge, core, discovery),
             CreateSenderParams { targets, options }: CreateSenderParams| {
                let bridge_ = bridge.clone();
                let discovery_ = discovery.clone();
                discovery.send(
                    targets,
                    Some(core.create_sender(&options, move || {
                        discovery_.send(Vec::new(), None);

                        let _ = bridge_.send("StatusChangeNotify");
                    })?),
                );

                bridge.send("StatusChangeNotify")?;
                Ok(())
            },
            (bridge.clone(), core.clone(), discovery.clone()),
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
             CreateReceiverParams { codec, description }: CreateReceiverParams| {
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
                            &description,
                            &HylaranaReceiverOptions {
                                video_decoder: codec,
                            },
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
            page: None,
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
            MessagePumpLoop::default()
                .create_runtime_attributes_builder::<NativeWindowWebView>()
                .with_browser_subprocess_path(&crate::APP_CONFIG.subprocess_path)
                .with_root_cache_path(&crate::APP_CONFIG.cache_path)
                .with_cache_path(&crate::APP_CONFIG.cache_path)
                .with_log_severity(LogLevel::Info)
                .with_custom_scheme(CustomSchemeAttributes::new(
                    "webview",
                    "localhost",
                    CustomRequestHandlerFactory::new(RequestHandlerWithLocalDisk::new(
                        &crate::APP_CONFIG.cheme_path,
                    )),
                ))
                .build()
                .create_runtime(IRuntimeObserver::new(self.events.clone()))?,
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

                    let page = runtime.create_webview(
                        &crate::APP_CONFIG.uri,
                        {
                            let size = window.inner_size();
                            WebViewAttributes {
                                window_handle: Some(window.window_handle()?.as_raw()),
                                width: size.width,
                                height: size.height,
                                ..Default::default()
                            }
                        },
                        IPageObserver::new(self.bridge.clone(), self.events.clone()),
                    )?;

                    let page = Arc::new(page);
                    let (tx, rx) = channel::<String>();
                    {
                        let page_ = page.clone();
                        thread::spawn(move || {
                            while let Ok(message) = rx.recv() {
                                page_.send_message(&message);
                            }
                        });
                    }

                    self.transport.write().replace(tx);
                    self.page.replace(page);
                }
            }
            UserEvents::OnRemoteWindowClose => {
                self.core.close_receiver();
                self.bridge.send("StatusChangeNotify")?;
            }
            UserEvents::OnMessagePumpPoll => {
                if self.runtime.is_some() {
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
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
                if self.runtime.is_some() {
                    MessagePumpLoop::default().poll();
                }
            }
            _ => (),
        }
    }

    pub fn about_to_wait(&self) {
        if self.runtime.is_some() {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }
}

struct IPageObserver {
    bridge: Arc<Bridge>,
    events: Arc<EventChannel>,
}

impl IPageObserver {
    fn new(bridge: Arc<Bridge>, events: Arc<EventChannel>) -> Self {
        Self { bridge, events }
    }
}

impl WebViewHandler for IPageObserver {
    fn on_message(&self, message: &str) {
        if let Err(e) = self.bridge.on_message(message.to_string()) {
            log::error!(
                "failed to handle message for wew webview observer, error={}",
                e
            );
        }
    }

    fn on_state_change(&self, state: WebViewState) {
        if state == WebViewState::Close {
            self.events.send_to_main(MainEvents::Shutdown);
        }
    }
}

struct IRuntimeObserver {
    events: Arc<EventChannel>,
    message_pump: Sender<u64>,
}

impl IRuntimeObserver {
    fn new(events: Arc<EventChannel>) -> Self {
        let (message_pump, rx) = channel::<u64>();
        let events_ = events.clone();
        thread::spawn(move || {
            while let Ok(delay) = rx.recv() {
                if delay > 0 {
                    thread::sleep(Duration::from_millis(delay));
                }

                events_.send(EventTarget::Frontend, UserEvents::OnMessagePumpPoll);
            }
        });

        Self {
            events,
            message_pump,
        }
    }
}

impl RuntimeHandler for IRuntimeObserver {
    fn on_context_initialized(&self) {
        self.events.send(
            EventTarget::Frontend,
            UserEvents::OnWebviewAppContextInitialized,
        );
    }
}

impl MessagePumpRuntimeHandler for IRuntimeObserver {
    fn on_schedule_message_pump_work(&self, delay: u64) {
        let _ = self.message_pump.send(delay);
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
    targets: Vec<String>,
    options: HylaranaSenderOptions,
}

#[derive(Deserialize)]
struct CreateReceiverParams {
    codec: VideoDecoderType,
    description: MediaStreamDescription,
}
