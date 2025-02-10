use std::{
    collections::HashMap,
    future::Future,
    str,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::{anyhow, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use tokio::{
    io::{stdin, stdout, AsyncReadExt, AsyncWriteExt, Stdout},
    sync::{
        mpsc::{unbounded_channel, UnboundedSender},
        oneshot::{channel, Sender},
        Mutex, RwLock,
    },
    time::timeout,
};

static LINE_START: &str = "::MESSAGE_TRANSPORTS-";

pub struct Route {
    sequence: AtomicU64,
    stdout: Mutex<Stdout>,
    // request sender table
    rst: Mutex<HashMap<u64, Sender<Value>>>,
    // on receiver table
    ort: RwLock<HashMap<String, UnboundedSender<(Sender<Result<Value>>, Value)>>>,
}

impl Route {
    async fn send(&self, message: String) {
        if self
            .stdout
            .lock()
            .await
            .write_all(format!("{}{}\n", LINE_START, message).as_bytes())
            .await
            .is_err()
        {
            log::error!("failed to send message to stdout!");
        }
    }

    pub async fn new() -> Arc<Self> {
        let this = Arc::new(Self {
            ort: RwLock::new(HashMap::with_capacity(100)),
            rst: Mutex::new(HashMap::with_capacity(100)),
            sequence: AtomicU64::new(0),
            stdout: Mutex::new(stdout()),
        });

        let this_ = Arc::downgrade(&this);
        tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];

            while let Ok(size) = stdin().read(&mut buf).await {
                if let Ok(line) = std::str::from_utf8(&buf[..size]) {
                    if line.starts_with(LINE_START) {
                        let (_, message) = line.split_at(LINE_START.len());

                        if let Some(this) = this_.upgrade() {
                            let _ = async {
                                match serde_json::from_str(&message)? {
                                    Payload::Request {
                                        method,
                                        sequence,
                                        content,
                                    } => {
                                        if let Some(sender) = this.ort.read().await.get(&method) {
                                            let (tx, rx) = channel();
                                            sender.send((tx, content))?;

                                            this.send(serde_json::to_string(&Payload::Response {
                                                content: ResponseContent::from(rx.await?),
                                                sequence,
                                            })?)
                                            .await;
                                        }
                                    }
                                    Payload::Response { sequence, content } => {
                                        if let Some(tx) = this.rst.lock().await.remove(&sequence) {
                                            let _ = tx.send(content);
                                        }
                                    }
                                }

                                Ok::<(), anyhow::Error>(())
                            }
                            .await;
                        } else {
                            break;
                        }
                    }
                }
            }
        });

        this
    }

    pub async fn call<Q, S>(&self, method: &str, content: Q) -> Result<S>
    where
        Q: Serialize,
        S: DeserializeOwned,
    {
        let sequence = self.sequence.fetch_add(1, Ordering::SeqCst);
        self.send(serde_json::to_string(&Payload::Request {
            method: method.to_string(),
            sequence,
            content,
        })?)
        .await;

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

    pub async fn on<T, Q, S, F, C>(&self, method: &str, handle: T, ctx: C)
    where
        T: Fn(C, Q) -> F + Send + Sync + 'static,
        Q: DeserializeOwned + Send,
        S: Serialize,
        F: Future<Output = Result<S>> + Send,
        C: Clone + Sync + Send + 'static,
    {
        let (tx, mut rx) = unbounded_channel();
        self.ort.write().await.insert(method.to_string(), tx);

        tokio::spawn(async move {
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
