use parking_lot::Mutex;
use std::{
    ffi::OsStr,
    io::{Read, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::{
        Arc,
        mpsc::{Receiver, TryRecvError, channel},
    },
    thread,
};

use crate::packets::{C2SPacket, S2CPacket};

pub struct Client {
    _proxy: Child,
    c2s: Arc<Mutex<Option<C2SPacket>>>,
    s2c: Receiver<S2CPacket>,
}
impl Default for Client {
    fn default() -> Self {
        Self::with_server("http://localhost:3000", SocketIo::Two)
    }
}

fn setup_proxy(
    mut stdin: ChildStdin,
    mut stdout: ChildStdout,
) -> (Receiver<S2CPacket>, Arc<Mutex<Option<C2SPacket>>>) {
    let (s2c_send, s2c_recv) = channel::<S2CPacket>();
    let c2s_arc1 = Arc::new(Mutex::new(None));
    let c2s_arc2 = Arc::clone(&c2s_arc1);
    let log_packets = std::env::var("LOG").is_ok_and(|v| !v.is_empty());
    // read packets from child stdout
    thread::spawn(move || {
        let mut command: Vec<u8> = vec![];
        let mut buf = [0];
        loop {
            match stdout.read(&mut buf) {
                Ok(1) => command.push(buf[0]),
                Err(e) => eprintln!("Error reading from stdout: {e}"),
                _ => (),
            }
            if buf[0] == b'\n' {
                let json_str =
                    String::from_utf8(std::mem::take(&mut command)).expect("incorrect string");
                let packet = serde_json::from_str(&json_str).expect("incorrect json");
                if log_packets {
                    println!("S -> C: {packet}");
                }
                s2c_send
                    .send(packet)
                    .expect("cannot send json to main thread");
            }
        }
    });

    // write packets to child stdin
    thread::spawn(move || {
        loop {
            if let Some(p) = c2s_arc1.lock().take() {
                let mut json = serde_json::to_string(&p).expect("cannot encode packet");
                // println!("{json}");
                json.push('\n');
                stdin
                    .write_all(json.as_bytes())
                    .expect("cannot send packet");
                if log_packets {
                    println!("S <- C: {p}");
                }
            }
        }
    });

    (s2c_recv, c2s_arc2)
}

pub enum SocketIo {
    Two,
    Four,
}

impl Client {
    pub fn with_server(server: impl AsRef<OsStr>, socketio_version: SocketIo) -> Self {
        let mut proxy = Command::new(
            #[cfg(target_os = "windows")]
            "./proxy.exe",
            #[cfg(not(target_os = "windows"))]
            "./proxy",
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .env("SERVER", server)
        .env(
            "MODERN",
            match socketio_version {
                SocketIo::Two => "",
                SocketIo::Four => "1",
            },
        )
        .spawn()
        .expect("cannot spawn proxy");

        let stdin = proxy.stdin.take().expect("no stdin");
        let stdout = proxy.stdout.take().expect("no stdout");

        let (s2c, c2s) = setup_proxy(stdin, stdout);

        println!("Client started!");

        Self {
            _proxy: proxy,
            c2s,
            s2c,
        }
    }

    pub fn send(&mut self, packet: C2SPacket) {
        _ = self.c2s.lock().insert(packet);
    }

    pub fn recv(&mut self) -> Option<S2CPacket> {
        match self.s2c.try_recv() {
            Ok(p) => Some(p),
            Err(TryRecvError::Empty) => None,
            _ => panic!("channel closed unexpectedly"),
        }
    }
}
