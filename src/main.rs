use std::{
    collections::HashMap,
    env,
    io::Error as IoError,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};

use tokio::net::{TcpListener, TcpStream};
use tungstenite::Message;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

const VERSION: u8 = 1;
const SET_LED: u8 = 0x10;
const INITIALIZE: u8 = 0x11;
const READY: u8 = 0x19;
const PING: u8 = 0x12;
const PONG: u8 = 0x1A;
const REQUEST_SERVER_INFO: u8 = 0xD0;
const REPORT_SERVER_INFO: u8 = 0xD8;
type HandlerReturnType = Option<Vec<u8>>;
fn ping_handler(payload: &[u8]) -> HandlerReturnType {
    let mut return_val = vec![VERSION, PONG];
    return_val.extend_from_slice(payload);
    return_val.extend([0x51, 0xED]);
    Some(return_val)
}

fn Initialize_handler(_: &[u8]) -> HandlerReturnType {
    let mut return_val = vec![VERSION, READY];
    Some(return_val)
}

fn set_led_handler(payload: &[u8]) -> HandlerReturnType {
    None
}

fn request_server_info_handler(payload: &[u8]) -> HandlerReturnType {
    let mut return_val = vec![VERSION, REPORT_SERVER_INFO];
    Some(return_val)
}

async fn handle_connection(raw_stream: TcpStream, addr: SocketAddr) {
    println!("Incoming TCP connection from: {}", addr);
    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("Error during the websocket handshake occurred");
    let (tx, rx) = unbounded();
    let (outgoing, incoming) = ws_stream.split();
    // peer_map.lock().unwrap().insert(addr, tx);
    let broadcast_incoming = incoming.try_for_each(move |msg| {
        let msg = dbg!(msg);
        println!(
            "Received a message from {}: {}",
            addr,
            msg.to_text().unwrap()
        );
        tx.unbounded_send(msg).unwrap();
        // let peers = peer_map.lock().unwrap();

        // We want to broadcast the message to everyone except ourselves.
        // let broadcast_recipients = peers
        //     .iter()
        //     .filter(|(peer_addr, _)| peer_addr != &&addr)
        //     .map(|(_, ws_sink)| ws_sink);

        // for recp in broadcast_recipients {
        //     recp.unbounded_send(msg.clone()).unwrap();
        // }

        future::ok(())
    });

    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;

    println!("{} disconnected", &addr);
}

#[tokio::main]
async fn main() -> Result<(), IoError> {
    println!("Hello, world!");
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8000".to_string());

    // let state = PeerMap::new(Mutex::new(HashMap::new()));
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    println!("listening on: {}", addr);

    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(stream, addr));
    }

    Ok(())
}
