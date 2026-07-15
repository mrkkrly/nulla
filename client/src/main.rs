use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;

use std::path::Path;
use nulla_core::keys::Identity;

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
                        println!("commands: help, post <text>, read, quit");
                    }
                    "post" => {
                        // STUB — next step builds + signs + sends an EVENT.
                        println!("(stub) would post: {rest}");
                    }
                    "read" => {
                        // STUB — next step sends a REQ.
                        println!("(stub) would send REQ");
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
                            println!("\n[relay] {text}\nnulla› ");
                        }
                    }
                    Some(Err(e)) => { println!("ws error: {e}"); break; }
                    None => { println!("relay disconnected"); break; }
                }
            }
        }
    }

}
