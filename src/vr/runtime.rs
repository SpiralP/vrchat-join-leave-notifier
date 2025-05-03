use std::{thread, time::Duration};

use anyhow::Result;
use openvr::{
    sys::{EVRNotificationStyle_None, EVRNotificationType_Transient},
    system::Event,
    ApplicationType,
};
use tokio::sync::{
    broadcast::{self, error::TryRecvError},
    mpsc,
};

use crate::notifier::{notify, NOTIFICATION};

pub async fn start_runtime(shutdown_send: broadcast::Sender<()>) -> Result<()> {
    let mut shutdown_recv = shutdown_send.subscribe();

    let mut vr_event_receiver = start_event_stream(shutdown_send.clone())?;

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown_recv.recv() => {
                    println!("start_runtime got shutdown");
                    break;
                },
                option = vr_event_receiver.recv() => {
                    if let Some(event) = option {
                        if let Event::Quit(_) = event {
                            break;
                        }
                    } else {
                        break;
                    }
                },
            }
        }

        println!("start_runtime end");
        let _ = shutdown_send.send(());
    });

    tokio::spawn(async {
        tokio::time::sleep(Duration::from_secs(3)).await;
        notify("init").await.unwrap();
    });

    Ok(())
}

fn start_event_stream(shutdown_send: broadcast::Sender<()>) -> Result<mpsc::Receiver<Event>> {
    let mut shutdown_recv = shutdown_send.subscribe();

    let (vr_event_sender, vr_event_receiver) = mpsc::channel(32);

    thread::spawn(move || {
        let mut do_loop = move || {
            let context = unsafe { openvr::init(ApplicationType::Overlay)? };
            let system = context.system()?;
            let overlay = context.overlay()?;
            let notifications = context.notifications()?;
            let notifications_overlay = overlay.find("system.systemui")?;

            let mut notification_receiver = {
                let guard = NOTIFICATION.lock().unwrap();
                let mut cell = guard.borrow_mut();
                cell.receiver.take().unwrap()
            };
            'outer: loop {
                if shutdown_recv
                    .try_recv()
                    .map(|_| true)
                    .unwrap_or_else(|e| e != TryRecvError::Empty)
                {
                    unsafe { context.shutdown() };
                    break 'outer;
                }

                loop {
                    match notification_receiver.try_recv() {
                        Ok(text) => {
                            notifications.create(
                                notifications_overlay,
                                0,
                                EVRNotificationType_Transient,
                                &text,
                                EVRNotificationStyle_None,
                                None,
                            )?;
                        }
                        Err(e) => {
                            if e != tokio::sync::mpsc::error::TryRecvError::Empty {
                                eprintln!("{e:?}");
                            }
                            break;
                        }
                    }
                }

                while let Some(event) = system.poll_next_event() {
                    let event = event.event;

                    vr_event_sender.blocking_send(event)?;

                    if let Event::Quit(_) = event {
                        // This extends the timeout until the process is killed
                        system.acknowledge_quit_exiting();

                        unsafe { context.shutdown() };
                        break 'outer;
                    }
                }

                thread::sleep(Duration::from_secs(1));
            }

            anyhow::Ok(())
        };

        if let Err(e) = do_loop() {
            eprintln!("start_event_stream: {e:?}");
        }
        println!("start_event_stream end");
        let _ = shutdown_send.send(());
    });

    Ok(vr_event_receiver)
}
