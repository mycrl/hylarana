use std::{
    collections::HashMap,
    io::{stdin, stdout, BufRead, Stdin, Stdout, Write},
    str,
    sync::{
        mpsc::{channel, Sender},
        Arc,
    },
    thread,
};

use anyhow::{anyhow, Result};
use parking_lot::RwLock;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

pub trait MessageTransport: Send + Sync {
    type Error;

    fn send(&self, message: &str) -> Result<(), Self::Error>;
    fn read<'a>(&self, message: &'a mut String) -> Result<&'a str, Self::Error>;
}

// This is a messaging layer using stdio.
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

    fn read<'a>(&self, line: &'a mut String) -> Result<&'a str, Self::Error> {
        self.stdin.lock().read_line(line)?;

        let message = line.trim();
        log::info!("stdio message transport recv message={}", message);

        Ok(message)
    }

    fn send(&self, message: &str) -> Result<(), Self::Error> {
        log::info!("stdio message transport send message={}", message);

        let mut stdout = self.stdout.lock();
        stdout.write(message.as_bytes())?;

        // It needs to be written on a line-by-line basis, so a newline character is
        // written here.
        stdout.write("\n".as_bytes())?;
        stdout.flush()?;

        Ok(())
    }
}

pub struct Route<M> {
    transport: M,
    table: RwLock<HashMap<String, Sender<(Sender<Result<Value>>, Value)>>>,
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
                if let Some(sender) = self.table.read().get(&method) {
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
            _ => (),
        }

        Ok(())
    }

    pub fn new(transport: M) -> Arc<Self> {
        let this = Arc::new(Self {
            table: RwLock::new(HashMap::with_capacity(100)),
            transport,
        });

        let this_ = Arc::downgrade(&this);
        thread::spawn(move || {
            let mut line = String::with_capacity(4096);

            while let Some(this) = this_.upgrade() {
                // To reuse the string buffer, empty the buffer contents before each use.
                line.clear();

                if let Ok(message) = this.transport.read(&mut line) {
                    if message.is_empty() {
                        break;
                    }

                    if let Err(e) = this.handle(message) {
                        log::error!(
                            "message router handle message error={}, message={}",
                            e,
                            message
                        );
                    }
                } else {
                    break;
                }
            }

            log::error!("stdin reader thread is closed");
        });

        this
    }

    pub fn send(&self, method: &str) -> Result<()> {
        self.transport
            .send(&serde_json::to_string(&Payload::<()>::Events {
                method: method.to_string(),
            })?)?;

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
    Events {
        method: String,
    },
}
