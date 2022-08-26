use std::{thread, time::Duration};

use anyhow::Result;
use openvr::{system::Event, ApplicationType};
use tokio::sync::{
    broadcast::{self, error::TryRecvError},
    mpsc,
};

pub async fn start_runtime(shutdown_send: broadcast::Sender<()>) -> Result<()> {
    let mut shutdown_recv = shutdown_send.subscribe();

    let mut stream = start_event_stream(shutdown_send.clone())?;

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = shutdown_recv.recv() => {
                    println!("start_runtime got shutdown");
                    break;
                },
                option = stream.recv() => {
                    if let Some(event) = option {
                        println!("{event:?}");

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

    Ok(())
}

fn start_event_stream(shutdown_send: broadcast::Sender<()>) -> Result<mpsc::Receiver<Event>> {
    let mut shutdown_recv = shutdown_send.subscribe();

    let (tx, rx) = mpsc::channel(32);

    thread::spawn(move || {
        let mut do_loop = move || {
            let context = unsafe { openvr::init(ApplicationType::Overlay)? };
            let system = context.system()?;

            'outer: loop {
                if shutdown_recv
                    .try_recv()
                    .map(|_| true)
                    .unwrap_or_else(|e| e != TryRecvError::Empty)
                {
                    unsafe { context.shutdown() };
                    break 'outer;
                }

                while let Some(event) = system.poll_next_event() {
                    let event = event.event;

                    tx.blocking_send(event)?;

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

    Ok(rx)
}
