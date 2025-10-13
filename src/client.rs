use std::{
    ffi::OsStr,
    io::{Read, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::mpsc::{Receiver, Sender, TryRecvError, channel},
    thread,
};

use crate::packets::{C2SPacket, S2CPacket};

pub struct Client {
    _proxy: Child,
    c2s: Sender<C2SPacket>,
    s2c: Receiver<S2CPacket>,
}
impl Default for Client {
    fn default() -> Self {
        Self::with_server("http://localhost:3000")
    }
}

fn setup_proxy(
    mut stdin: ChildStdin,
    mut stdout: ChildStdout,
) -> (Receiver<S2CPacket>, Sender<C2SPacket>) {
    let (s2c_send, s2c_recv) = channel::<S2CPacket>();
    let (c2s_send, c2s_recv) = channel::<C2SPacket>();
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
                println!("S -> C: {packet:?}");
                s2c_send
                    .send(packet)
                    .expect("cannot send json to main thread");
            }
        }
    });

    // write packets to child stdin
    thread::spawn(move || {
        loop {
            match c2s_recv.recv() {
                Ok(p) => {
                    let mut json = serde_json::to_string(&p).expect("cannot encode packet");
                    json.push('\n');
                    stdin
                        .write_all(json.as_bytes())
                        .expect("cannot send packet");
                    println!("S <- C: {p:?}");
                }
                Err(e) => eprintln!("Error reading from packet channel: {e}"),
            }
        }
    });

    (s2c_recv, c2s_send)
}

impl Client {
    pub fn with_server(server: impl AsRef<OsStr>) -> Self {
        let mut proxy = Command::new("bun")
            .args(["run", "src/proxy.ts"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .env("SERVER", server)
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
        self.c2s
            .send(packet)
            .expect("failed to send; channel closed");
    }

    pub fn recv(&mut self) -> Option<S2CPacket> {
        match self.s2c.try_recv() {
            Ok(p) => Some(p),
            Err(TryRecvError::Empty) => None,
            _ => panic!("channel closed unexpectedly"),
        }
    }
}
