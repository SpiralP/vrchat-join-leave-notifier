use std::{
    io::{BufReader, Cursor},
    sync::{mpsc, OnceLock},
    thread,
    time::Duration,
};

use anyhow::{ensure, Result};
use rodio::{Decoder, OutputStream, Sink};
use tokio::sync::{broadcast, oneshot};

const JOIN_SOUND_BYTES: &[u8] = include_bytes!("../../sounds/mixkit-correct-answer-tone-2870.wav");
const LEAVE_SOUND_BYTES: &[u8] =
    include_bytes!("../../sounds/mixkit-software-interface-back-2575.wav");

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum AudioEvent {
    Join,
    Leave,
}

static EVENTS_TX: OnceLock<mpsc::Sender<AudioEvent>> = OnceLock::new();

pub async fn start_audio(shutdown_send: broadcast::Sender<()>) -> Result<()> {
    let mut shutdown_recv = shutdown_send.subscribe();

    let (events_tx, events_rx) = mpsc::channel();
    EVENTS_TX.set(events_tx).unwrap();

    let (ok_send, ok_recv) = oneshot::channel();

    thread::spawn(move || {
        let (stream, stream_handle) = match OutputStream::try_default() {
            Ok(stream) => {
                ok_send.send(true).unwrap();
                stream
            }
            Err(e) => {
                eprintln!("start_audio: {e}");
                ok_send.send(false).unwrap();
                return;
            }
        };

        let loop_result = (move || {
            loop {
                let event = match events_rx.recv_timeout(Duration::from_secs(1)) {
                    Ok(event) => event,
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if shutdown_recv
                            .try_recv()
                            .map_or_else(|e| e != broadcast::error::TryRecvError::Empty, |()| true)
                        {
                            println!("start_audio got shutdown");
                            break;
                        }
                        continue;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        eprintln!("start_audio events_rx disconnected");
                        break;
                    }
                };

                let sink = Sink::try_new(&stream_handle)?;
                sink.set_volume(0.2);
                sink.append(Decoder::new(BufReader::new(Cursor::new(match event {
                    AudioEvent::Join => JOIN_SOUND_BYTES,
                    AudioEvent::Leave => LEAVE_SOUND_BYTES,
                })))?);
                sink.detach();
            }

            drop(stream);
            drop(stream_handle);

            anyhow::Ok(())
        })();

        if let Err(e) = loop_result {
            eprintln!("start_audio: {e:?}");
        }
        println!("start_audio end");
        let _ = shutdown_send.send(());
    });

    let ok = ok_recv.await?;
    ensure!(ok, "start_audio failed");

    Ok(())
}

pub fn handle_event(event: &AudioEvent) -> Result<()> {
    EVENTS_TX
        .get()
        .ok_or_else(|| anyhow::anyhow!("Audio events channel not initialized"))?
        .send(event.clone())
        .map_err(|e| anyhow::anyhow!("Failed to send audio event: {e}"))?;
    Ok(())
}
