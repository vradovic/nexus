import $ from "jquery";
import "@chrisoakman/chessboardjs/dist/chessboard-1.0.0.css";
import chessboardScriptUrl from "@chrisoakman/chessboardjs/dist/chessboard-1.0.0.js?url";
import "./styles.css";

window.$ = $;
window.jQuery = $;

const START_POSITION = "start";
const DEFAULT_WS_URL = "ws://127.0.0.1:3000/ws";
const PIECE_THEME_URL = "https://chessboardjs.com/img/chesspieces/wikipedia/{piece}.png";

const form = document.querySelector("#connect-form");
const wsUrlInput = document.querySelector("#ws-url");
const connectButton = document.querySelector("#connect-button");
const resetButton = document.querySelector("#reset-button");
const flipButton = document.querySelector("#flip-button");
const statusDot = document.querySelector("#status-dot");
const statusText = document.querySelector("#status-text");
const clientLabel = document.querySelector("#client-label");
const clientColor = document.querySelector("#client-color");
const logList = document.querySelector("#log");
const positionLabel = document.querySelector("#position-label");
const lastEvent = document.querySelector("#last-event");
const moveCount = document.querySelector("#move-count");

const clientId = getOrCreateClientId();
const clientName = `player-${clientId.slice(0, 6)}`;
const color = colorFromId(clientId);

let board = null;
let socket = null;
let currentPosition = START_POSITION;
let moves = 0;

wsUrlInput.value = localStorage.getItem("nexus-demo-ws-url") || DEFAULT_WS_URL;
clientLabel.textContent = clientName;
clientColor.style.background = color;

form.addEventListener("submit", (event) => {
  event.preventDefault();
  connect(wsUrlInput.value.trim());
});

resetButton.addEventListener("click", () => {
  setBoardPosition(START_POSITION, true);
  moves = 0;
  moveCount.textContent = "0";
  sendBoardPosition({
    action: "reset",
    move: null,
  });
});

flipButton.addEventListener("click", () => {
  board?.flip();
});

window.addEventListener("resize", () => {
  board?.resize();
});

initializeBoard().catch((error) => {
  setStatus("disconnected", "Board error");
  writeLog(error.message);
});

async function initializeBoard() {
  const Chessboard = await loadChessboard();

  board = Chessboard("board", {
    draggable: true,
    dropOffBoard: "snapback",
    position: currentPosition,
    pieceTheme: PIECE_THEME_URL,
    showErrors: "console",
    onDrop: handleDrop,
  });

  writeLog("board ready");
}

function handleDrop(source, target, piece, newPosition) {
  if (target === "offboard") {
    return "snapback";
  }

  if (!isSocketOpen()) {
    writeLog("connect before moving pieces");
    return "snapback";
  }

  const nextPosition = clonePosition(newPosition);
  currentPosition = nextPosition;
  updatePositionLabel(nextPosition);
  moves += 1;
  moveCount.textContent = String(moves);

  sendBoardPosition({
    action: "move",
    move: { source, target, piece },
    position: nextPosition,
  });

  return undefined;
}

function connect(url) {
  if (!url) return;

  if (socket) {
    socket.close();
    socket = null;
  }

  localStorage.setItem("nexus-demo-ws-url", url);
  setStatus("connecting", "Connecting");

  let nextSocket;

  try {
    nextSocket = new WebSocket(url);
  } catch (error) {
    setStatus("disconnected", "Invalid URL");
    writeLog(error.message);
    return;
  }

  nextSocket.binaryType = "arraybuffer";
  socket = nextSocket;

  nextSocket.addEventListener("open", () => {
    if (socket !== nextSocket) return;

    setStatus("connected", "Connected");
    connectButton.textContent = "Reconnect";
    writeLog(`connected as ${clientName}`);

    sendPayload({
      type: "chess.sync_request",
      clientId,
      clientName,
      color,
      createdAt: Date.now(),
    });
  });

  nextSocket.addEventListener("message", async (event) => {
    if (socket !== nextSocket) return;

    const payload = await decodeMessage(event.data);
    if (!payload) return;
    applyPayload(payload);
  });

  nextSocket.addEventListener("close", () => {
    if (socket !== nextSocket) return;

    setStatus("disconnected", "Disconnected");
    writeLog("socket closed");
  });

  nextSocket.addEventListener("error", () => {
    if (socket !== nextSocket) return;

    setStatus("disconnected", "Connection error");
    writeLog("socket error");
  });
}

function sendBoardPosition({ action, move, position = currentPosition }) {
  return sendPayload({
    type: "chess.position",
    action,
    clientId,
    clientName,
    color,
    move,
    moves,
    position: clonePosition(position),
    updatedAt: Date.now(),
  });
}

function sendPayload(payload) {
  if (!isSocketOpen()) {
    writeLog("socket is not connected");
    return false;
  }

  socket.send(JSON.stringify(payload));
  return true;
}

async function decodeMessage(data) {
  let text;

  if (typeof data === "string") {
    text = data;
  } else if (data instanceof ArrayBuffer) {
    text = new TextDecoder().decode(data);
  } else if (data instanceof Blob) {
    text = await data.text();
  } else {
    writeLog("received unsupported message type");
    return null;
  }

  try {
    return JSON.parse(text);
  } catch {
    writeLog(`received non-json payload: ${text.slice(0, 80)}`);
    return null;
  }
}

function applyPayload(payload) {
  if (!payload || typeof payload !== "object") return;

  lastEvent.textContent = payload.type || "unknown";

  if (payload.type === "chess.sync_request") {
    if (payload.clientId !== clientId) {
      sendBoardPosition({
        action: "sync_response",
        move: null,
      });
      writeLog(`${payload.clientName || "player"} joined`);
    }
    return;
  }

  if (payload.type === "chess.position") {
    if (!isValidPosition(payload.position)) {
      writeLog("received invalid board position");
      return;
    }

    setBoardPosition(payload.position, true);
    moves = Number.isFinite(payload.moves) ? payload.moves : moves;
    moveCount.textContent = String(moves);
    writeMoveLog(payload);
    return;
  }

  writeLog(`received ${payload.type || "unknown"} payload`);
}

function setBoardPosition(position, animate) {
  const nextPosition = clonePosition(position);
  currentPosition = nextPosition;
  board?.position(nextPosition, animate);
  updatePositionLabel(nextPosition);
}

function updatePositionLabel(position) {
  if (position === START_POSITION) {
    positionLabel.textContent = "start";
    return;
  }

  positionLabel.textContent = `${Object.keys(position).length} pieces`;
}

function writeMoveLog(payload) {
  if (payload.action === "reset") {
    writeLog(`${payload.clientName || "player"} reset the board`);
    return;
  }

  if (payload.action === "sync_response") {
    writeLog(`synced from ${payload.clientName || "player"}`);
    return;
  }

  if (payload.move) {
    const { source, target, piece } = payload.move;
    writeLog(`${payload.clientName || "player"} moved ${piece} ${source}-${target}`);
    return;
  }

  writeLog(`${payload.clientName || "player"} updated the board`);
}

function isSocketOpen() {
  return socket?.readyState === WebSocket.OPEN;
}

function clonePosition(position) {
  if (typeof position === "string") {
    return position;
  }

  return { ...position };
}

function isValidPosition(position) {
  return position === START_POSITION || (position && typeof position === "object" && !Array.isArray(position));
}

async function loadChessboard() {
  if (!window.Chessboard) {
    await loadScript(chessboardScriptUrl);
  }

  if (typeof window.Chessboard !== "function") {
    throw new Error("chessboard.js did not expose window.Chessboard");
  }

  return window.Chessboard;
}

function loadScript(src) {
  return new Promise((resolve, reject) => {
    const script = document.createElement("script");
    script.src = src;
    script.async = true;
    script.addEventListener("load", resolve, { once: true });
    script.addEventListener("error", () => reject(new Error(`failed to load ${src}`)), { once: true });
    document.head.append(script);
  });
}

function getOrCreateClientId() {
  const existing = sessionStorage.getItem("nexus-demo-client-id");
  if (existing) return existing;

  const created = createId();
  sessionStorage.setItem("nexus-demo-client-id", created);
  return created;
}

function createId() {
  if (crypto.randomUUID) {
    return crypto.randomUUID();
  }

  return `client-${Math.random().toString(16).slice(2)}-${Date.now().toString(16)}`;
}

function setStatus(kind, text) {
  statusDot.className = `status-dot ${kind}`;
  statusText.textContent = text;
}

function writeLog(message) {
  const item = document.createElement("li");
  item.textContent = message;
  logList.prepend(item);

  while (logList.children.length > 14) {
    logList.lastChild.remove();
  }
}

function colorFromId(id) {
  let hash = 0;
  for (let index = 0; index < id.length; index += 1) {
    hash = (hash * 31 + id.charCodeAt(index)) % 360;
  }
  return `hsl(${hash} 76% 46%)`;
}
