use std::{cell::RefCell, collections::HashSet, fmt::Display, time::Duration};

use anyhow::Result;
use deunicode::deunicode;
use tokio::sync::{mpsc, Mutex, MutexGuard, OnceCell};

pub struct Notification {
    pub sender: mpsc::Sender<String>,
    pub receiver: Option<mpsc::Receiver<String>>,
}

impl Notification {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(32);
        Self {
            sender,
            receiver: Some(receiver),
        }
    }
}

impl Default for Notification {
    fn default() -> Self {
        Self::new()
    }
}

lazy_static::lazy_static!(
    pub static ref NOTIFICATION: std::sync::Mutex<RefCell<Notification>> =
    std::sync::Mutex::new(RefCell::new(Notification::new()));
);

pub async fn notify(text: &str) -> Result<()> {
    let text = deunicode(text);
    println!("{text}");

    let sender = {
        let guard = NOTIFICATION.lock().unwrap();
        let cell = guard.borrow_mut();
        cell.sender.clone()
    };

    sender.send(text).await?;

    Ok(())
}

static DEBOUNCED: OnceCell<Mutex<HashSet<MessageEvent>>> = OnceCell::const_new();

async fn with_debounced<F, R>(f: F) -> R
where
    F: FnOnce(MutexGuard<HashSet<MessageEvent>>) -> R,
{
    let debounced = DEBOUNCED
        .get_or_init(move || async move { Default::default() })
        .await;
    let debounced = debounced.lock().await;
    f(debounced)
}

const DEBOUNCE_DURATION: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum MessageEvent {
    Join(String),
    Leave(String),
}

impl Display for MessageEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Join(name) => write!(f, "{name} joined"),
            Self::Leave(name) => write!(f, "{name} left"),
        }
    }
}

pub async fn debounced_notify(event: MessageEvent) -> Result<()> {
    with_debounced(move |mut debounced| {
        let was_empty = debounced.is_empty();

        debounced.insert(event);

        if was_empty {
            tokio::spawn(async move {
                tokio::time::sleep(DEBOUNCE_DURATION).await;

                let notifies: Vec<(String, Option<Vec<String>>)> =
                    with_debounced(move |mut debounced| {
                        let mut notifies = Vec::new();

                        let mut join_messages = Vec::new();
                        let mut leave_messages = Vec::new();
                        for event in debounced.drain() {
                            match event {
                                MessageEvent::Join(_) => join_messages.push(event.to_string()),
                                MessageEvent::Leave(_) => leave_messages.push(event.to_string()),
                            }
                        }

                        if !join_messages.is_empty() {
                            notifies.push(group(join_messages, "players joined"));
                        }

                        if !leave_messages.is_empty() {
                            notifies.push(group(leave_messages, "players left"));
                        }

                        notifies
                    })
                    .await;

                for (title, _body_lines) in notifies {
                    if let Err(e) = notify(&title).await {
                        eprintln!("{e:?}");
                    }
                }
            });
        }

        Ok(())
    })
    .await
}

fn group(mut messages: Vec<String>, suffix: &str) -> (String, Option<Vec<String>>) {
    if messages.len() == 1 {
        (messages.remove(0), None)
    } else {
        let title = format!("{} {suffix}", messages.len());

        if messages.len() < 7 {
            (title, Some(messages))
        } else {
            // show counts instead of names
            (title, None)
        }
    }
}
