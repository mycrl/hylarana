use std::{
    collections::HashMap,
    io::{stdin, stdout, BufRead, Stdin, Stdout, Write},
    str,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{channel, Sender},
        Arc,
    },
    thread,
    time::Duration,
};

use anyhow::{anyhow, Result};
use parking_lot::{Mutex, RwLock};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use tokio::{sync::oneshot, time::timeout};

use crate::RUNTIME;

pub trait MessageTransport: Send + Sync {
    type Error;

    fn send(&self, message: &str) -> Result<(), Self::Error>;
    fn read(&self, message: &mut String) -> Result<(), Self::Error>;
}

pub struct Stdio {
    stdin: Stdin,
    stdout: Stdout,
}

impl Default for Stdio {
    fn default() -> Self {
        Self {
            stdin: stdin(),
            stdout: stdout(),
        }
    }
}

impl MessageTransport for Stdio {
    type Error = anyhow::Error;

    fn read(&self, message: &mut String) -> Result<(), Self::Error> {
        self.stdin.lock().read_line(message)?;

        log::info!("stdio message transport recv message={}", message.trim());

        Ok(())
    }

    fn send(&self, message: &str) -> Result<(), Self::Error> {
        log::info!("stdio message transport send message={}", message);

        let mut stdout = self.stdout.lock();
        stdout.write(message.as_bytes())?;
        stdout.write("\n".as_bytes())?;
        stdout.flush()?;

        Ok(())
    }
}

pub struct Route<M> {
    transport: M,
    sequence: AtomicU64,
    // request sender table
    rst: Mutex<HashMap<u64, oneshot::Sender<Value>>>,
    // on receiver table
    ort: RwLock<HashMap<String, Sender<(Sender<Result<Value>>, Value)>>>,
}

impl<M> Route<M>
where
    M: MessageTransport<Error = anyhow::Error> + 'static,
{
    fn handle(&self, message: &str) -> Result<()> {
        match serde_json::from_str(message)? {
            Payload::Request {
                method,
                sequence,
                content,
            } => {
                if let Some(sender) = self.ort.read().get(&method) {
                    let (tx, rx) = channel();
                    sender.send((tx, content))?;

                    if let Err(e) =
                        self.transport
                            .send(&serde_json::to_string(&Payload::Response {
                                content: ResponseContent::from(rx.recv()?),
                                sequence,
                            })?)
                    {
                        log::error!("failed to send message to stdout, error={}", e);
                    }
                }
            }
            Payload::Response { sequence, content } => {
                if let Some(tx) = self.rst.lock().remove(&sequence) {
                    let _ = tx.send(content);
                }
            }
        }

        Ok(())
    }

    pub fn new(transport: M) -> Arc<Self> {
        let this = Arc::new(Self {
            ort: RwLock::new(HashMap::with_capacity(100)),
            rst: Mutex::new(HashMap::with_capacity(100)),
            sequence: AtomicU64::new(0),
            transport,
        });

        let this_ = Arc::downgrade(&this);
        thread::spawn(move || {
            let mut message = String::with_capacity(4096);

            while let Some(this) = this_.upgrade() {
                message.clear();

                if this.transport.read(&mut message).is_ok() {
                    if let Err(e) = this.handle(&message) {
                        log::error!("message router handle message error={}", e)
                    }
                } else {
                    break;
                }
            }

            log::error!("stdin reader thread is closed");
        });

        this
    }

    pub async fn call<Q, S>(&self, method: &str, content: Q) -> Result<S>
    where
        Q: Serialize,
        S: DeserializeOwned,
    {
        let sequence = self.sequence.fetch_add(1, Ordering::SeqCst);
        self.transport
            .send(&serde_json::to_string(&Payload::Request {
                method: method.to_string(),
                sequence,
                content,
            })?)?;

        let (tx, rx) = oneshot::channel();
        self.rst.lock().insert(sequence, tx);

        let response = match timeout(Duration::from_secs(5), rx).await {
            Err(_) | Ok(Err(_)) => {
                drop(self.rst.lock().remove(&sequence));

                return Err(anyhow!("request timeout"));
            }
            Ok(Ok(it)) => it,
        };

        let response: ResponseContent<S> = serde_json::from_value(response)?;
        response.into()
    }

    pub fn send_event(self: &Arc<Self>, event: &'static str) {
        let this = self.clone();
        RUNTIME.spawn(async move {
            let _ = this.call::<_, ()>(event, ()).await;
        });
    }

    pub fn on<T, Q, S, C>(&self, method: &str, handle: T, ctx: C)
    where
        T: Fn(C, Q) -> Result<S> + Send + 'static,
        Q: DeserializeOwned + Send,
        S: Serialize,
        C: Clone + Send + 'static,
    {
        let (tx, rx) = channel();
        self.ort.write().insert(method.to_string(), tx);

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
