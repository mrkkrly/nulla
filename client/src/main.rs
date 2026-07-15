use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;

use std::path::Path;
use nulla_core::{event::Event, keys::Identity, message::{CloseMsg, EventMsg, Filter, ReqMsg}};

fn load_or_create_identity(path: &str) -> Identity {
    if Path::new(path).exists() {
        let hex = std::fs::read_to_string(path)
            .expect("ready key file")
            .trim()
            .to_string();

        match Identity::from_secret_hex(&hex) {
            Some(id) => {
                println!("loaded identity from {path}");
                id
            }
            None => {
                panic!("key file {path} is corrupt — delete it to regenerate");
            }
        }
    } else {
        let id = Identity::generate();
        std::fs::write(path, id.secret_key_hex()).expect("write key file");
        println!("Generated new identity, saved to {path}");
        id
    }
}

#[tokio::main]
async fn main() {
    let url = "ws://127.0.0.1:8080/";
    let (ws, _) = connect_async(url).await.expect("connect failed");
    let (mut ws_write, mut ws_read) = ws.split();
    println!("Connected to {url}. Type `help`, or `quit` to exit.");

    let identity = load_or_create_identity("nulla_key.hex");
    println!("pubkey: {}", identity.public_key_hex());

    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<String>();

    std::thread::spawn(move || {
       let mut rl = rustyline::DefaultEditor::new().expect("editor");
       loop {
           match rl.readline("nulla› ") {
               Ok(line) => {
                   let _ = rl.add_history_entry(line.as_str());
                   if cmd_tx.send(line).is_err() {
                       break;
                   }
               }
               Err(_) => {
                   let _ = cmd_tx.send("quit".to_string());
                   break;
               }
           }
       }
    });

    loop {
        tokio::select! {
            maybe_line = cmd_rx.recv() => {
                let Some(line) = maybe_line else { break };
                let line = line.trim();
                if line.is_empty() { continue; }

                let mut parts = line.splitn(2, ' ');
                let cmd = parts.next().unwrap_or("");
                let rest = parts.next().unwrap_or("").trim();

                match cmd {
                    "quit" | "exit" => {
                        println!("bye");
                        break;
                    }
                    "help" => {
                        println!("commands:");
                        println!("  post <text>              publish a note");
                        println!("  sub <name> <filter>      subscribe, e.g. sub feed {{\"kinds\":[1]}}");
                        println!("  close <name>             cancel a subscription");
                        println!("  quit                     exit");
                    }
                    "post" => {
                        if rest.is_empty() {
                            println!("usage: post <text>");
                        } else {
                            let ev = Event::create(&identity, 1, rest);
                            let msg = EventMsg("EVENT".to_string(), ev);
                            let json = serde_json::to_string(&msg).unwrap();
                            if let Err(e) = ws_write.send(json.into()).await {
                                println!("send failed: {e}");
                            }
                        }
                        // STUB — next step builds + signs + sends an EVENT.

                    }
                    "sub" => {
                        let mut sp = rest.splitn(2, ' ');
                        let name = sp.next().unwrap_or("").trim();
                        let filter_str = sp.next().unwrap_or("").trim();

                        if name.is_empty() {
                            println!("usage: sub <name> <filter-json>   e.g. sub feed {{\"kinds\":[1]}}");
                        } else {
                            let filter_json = if filter_str.is_empty() { "{}" } else { filter_str };
                            match serde_json::from_str::<Filter>(filter_json) {
                                Ok(filter) => {
                                    let req = ReqMsg("REQ".to_string(), name.to_string(), filter);
                                    let json = serde_json::to_string(&req).unwrap();
                                    let _ = ws_write.send(json.into()).await;
                                    println!("subscribed {name}");
                                }
                                Err(e) => println!("bad filter json: {e}")
                            }
                        }
                    }
                    "read" => {
                        // STUB — next step sends a REQ.
                        println!("(stub) would send REQ");
                    }
                    "close" => {
                        if rest.is_empty() {
                            println!("usage: close <name>")
                        } else {
                            let c = CloseMsg("CLOSE".to_string(), rest.to_string());
                            let json = serde_json::to_string(&c).unwrap();
                            let _ = ws_write.send(json.into()).await;
                            println!("closing: {rest}")
                        }

                    }
                    other => {
                        println!("unknown command: {other}");
                    }
                }
            }

            maybe_msg = ws_read.next() => {
                match maybe_msg {
                    Some(Ok(msg)) => {
                        if let Ok(text) = msg.to_text() {
                            // println!("\n[relay] {text}\nnulla› ");
                            if let Ok(val) = serde_json::from_str::<serde_json::Value>(text) {
                                if val[0] == "EVENT" {
                                    let sub = val[1].as_str().unwrap_or("?");
                                    let content = val[2]["content"].as_str().unwrap_or("");
                                    let pk = val[2]["pubkey"].as_str().unwrap_or("");
                                    let short = &pk[..pk.len().min(8)];
                                    println!("\n[{sub}] {short}: {content}");
                                    print!("nulla› ");
                                    use std::io::Write;
                                    let _ = std::io::stdout().flush();
                                    continue;
                                }
                            }
                            println!("\n[relay] {text}");
                            print!("nulla› ");
                            use std::io::Write;
                            let _ = std::io::stdout().flush();
                        }
                    }
                    Some(Err(e)) => { println!("ws error: {e}"); break; }
                    None => { println!("relay disconnected"); break; }
                }
            }
        }
    }

}
