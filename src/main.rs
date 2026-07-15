use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::sync::{Mutex, broadcast};
use tokio_tungstenite::accept_async;
use futures_util::{StreamExt, SinkExt};

mod event;
mod keys;
mod message;
mod db;

use event::Event;
use keys::Identity;
use message::{ClientMessage, EventMsg, ReqMsg};
use db::Db;

#[tokio::main]
async fn main() {
    // --- Throwaway: generate a valid ["EVENT", {...}] for testing. ---
    // Uncomment, run once, copy the printed line into websocat, then re-comment.
    //
    let tmp = Identity::generate();
    let ev = Event::create(&tmp, 1, "hello");
    println!("{}", serde_json::to_string(&EventMsg("EVENT".into(), ev)).unwrap());

    // Shared in-memory event store (resets on restart).
    let store: Arc<Mutex<Db>> = Arc::new(Mutex::new(Db::open("relay.db")));

    // Broadcast channel: one connection's EVENT reaches all open REQs.
    let (tx, _rx) = broadcast::channel::<Event>(100);

    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    println!("Listening on ws://127.0.0.1:8080");

    while let Ok((stream, _)) = listener.accept().await {
        let store = Arc::clone(&store);
        let tx = tx.clone();

        tokio::spawn(async move {
            let ws = match accept_async(stream).await {
                Ok(ws) => ws,
                Err(e) => {
                    eprintln!("handshake failed: {}", e);
                    return;
                }
            };

            // Split so we can read client input and write pushes independently.
            let (mut write, mut read) = ws.split();
            let mut rx = tx.subscribe();

            // This connection's current subscription (one at a time for now).
            let mut active_sub: Option<(String, message::Filter)> = None;

            loop {
                tokio::select! {
                    // Branch 1: the client sent us a message.
                    maybe_msg = read.next() => {
                        let Some(Ok(msg)) = maybe_msg else { break };
                        if let Ok(text) = msg.to_text() {
                            match serde_json::from_str::<ClientMessage>(text) {
                                Ok(ClientMessage::Event(EventMsg(_, ev))) => {
                                    if ev.verify() {
                                        let id = ev.id.clone();
                                        store.lock().await.insert(&ev);
                                        let _ = tx.send(ev);
                                        let _ = write
                                            .send(format!("OK: stored {}", id).into())
                                            .await;
                                    } else {
                                        let _ = write
                                            .send("REJECTED: bad signature".into())
                                            .await;
                                    }
                                }
                                Ok(ClientMessage::Req(ReqMsg(_, sub_id, filter))) => {
                                    // Replay stored matches, then EOSE.
                                    let matches = store.lock().await.query(&filter);
                                    for ev in matches {
                                        let out = serde_json::to_string(
                                            &("EVENT", &sub_id, ev)
                                        ).unwrap();
                                        let _ = write.send(out.into()).await;
                                    }
                                    // drop(events);
                                    let _ = write
                                        .send(format!("[\"EOSE\",\"{}\"]", sub_id).into())
                                        .await;
                                    // Remember it for live pushes.
                                    active_sub = Some((sub_id, filter));
                                }
                                Err(e) => {
                                    let _ = write
                                        .send(format!("ERROR: {}", e).into())
                                        .await;
                                }
                            }
                        }
                    }

                    // Branch 2: a new event was broadcast by some connection.
                    Ok(ev) = rx.recv() => {
                        if let Some((sub_id, filter)) = &active_sub {
                            if filter.matches(&ev) {
                                let out = serde_json::to_string(
                                    &("EVENT", sub_id, &ev)
                                ).unwrap();
                                let _ = write.send(out.into()).await;
                            }
                        }
                    }
                }
            }
        });
    }
}
