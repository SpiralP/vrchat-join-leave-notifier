use std::{collections::HashSet, fmt::Display, time::Duration};

use anyhow::Result;
use deunicode::deunicode;
use tokio::sync::{Mutex, MutexGuard, OnceCell};
use xsoverlay_notifications::{Notification, XSOverlayNotifier};

static NOTIFIER: OnceCell<XSOverlayNotifier> = OnceCell::const_new();

// can only see 7 lines at a time
const MAX_BODY_LINES: usize = 7;

async fn notify(title: &str, body_lines: Option<Vec<String>>) -> Result<()> {
    let notifier = NOTIFIER
        .get_or_try_init(move || async move { XSOverlayNotifier::new().await })
        .await?;

    let title = deunicode(title);
    let body = body_lines
        .as_ref()
        .map(|body_lines| format!("<size=20>{}", deunicode(&body_lines.join("\n"))))
        .unwrap_or_default();
    println!("{title}");
    println!("{body}");

    notifier
        .send(&Notification {
            message_type: 1,
            timeout: 2.0 + body_lines.map(|lines| lines.len() as f32).unwrap_or(0.0) * 0.5,
            opacity: 0.2,
            title: title.to_owned(),
            content: body.to_owned(),
            height: 250.0,
            ..Default::default()
        })
        .await?;

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

                for (title, body_lines) in notifies {
                    if let Err(e) = notify(&title, body_lines).await {
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

        if messages.len() < MAX_BODY_LINES {
            (title, Some(messages))
        } else {
            // show counts instead of names
            (title, None)
        }
    }
}
