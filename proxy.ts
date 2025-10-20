import { connect } from "socket.io-client";
import { io, Socket } from "socket.io-client-v4";

const modern =
  typeof process.env.MODERN == "string" && process.env.MODERN.length > 0;

var socket: Socket;
if (modern) {
  socket = io(process.env.SERVER || "https://blockly.kbylabs.com", {
    port: "443",
    transports: ["websocket"],
    path: "/socket.io",
    secure: true,
  });
} else {
  socket = connect(process.env.SERVER || "http://localhost:3000");
}

type SocketPacket = {
  packet: string;
  data: object;
};
const log = (p: string) => (msg: object) =>
  process.stdout.write(`${JSON.stringify({ packet: p, data: msg })}\n`);
const serverToClientPackets = [
  "joined_room",
  "updata_board",
  "new_board",
  "game_result",
  "get_ready_rec",
  "move_rec",
  "put_rec",
  "look_rec",
  "search_rec",
  "match_init_rec",
  "match_start_check_rec",
  "error",
  "connect_error",
];

serverToClientPackets.forEach((p) => socket.on(p, log(p)));

// while (!socket.connected) {}
// console.error("active");

for await (const line of console) {
  try {
    const packet: SocketPacket = JSON.parse(line);
    socket.emit(packet.packet, packet.data);
  } catch {
    break;
  }
}
