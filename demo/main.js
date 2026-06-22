import $ from "jquery";
import "@chrisoakman/chessboardjs/dist/chessboard-1.0.0.css";
import chessboardScriptUrl from "@chrisoakman/chessboardjs/dist/chessboard-1.0.0.js?url";
import "./styles.css";

window.$ = $;
window.jQuery = $;

const START_POSITION = "start";
const AUTH_URL = "http://127.0.0.1:3001";
const WS_URL = "ws://127.0.0.1:3000/ws";
const TOKEN_STORAGE_KEY = "nexus-demo-access-token";
const PROFILE_STORAGE_KEY = "nexus-demo-profile";
const PIECE_THEME_URL = "https://chessboardjs.com/img/chesspieces/wikipedia/{piece}.png";

const loginPage = document.querySelector("#login-page");
const registerPage = document.querySelector("#register-page");
const gamePage = document.querySelector("#game-page");

const loginForm = document.querySelector("#login-form");
const loginEmailInput = document.querySelector("#login-email");
const loginPasswordInput = document.querySelector("#login-password");
const loginError = document.querySelector("#login-error");
const showRegisterButton = document.querySelector("#show-register-button");

const registerForm = document.querySelector("#register-form");
const registerEmailInput = document.querySelector("#register-email");
const registerUsernameInput = document.querySelector("#register-username");
const registerFirstNameInput = document.querySelector("#register-first-name");
const registerLastNameInput = document.querySelector("#register-last-name");
const registerPasswordInput = document.querySelector("#register-password");
const registerError = document.querySelector("#register-error");
const showLoginButton = document.querySelector("#show-login-button");

const logoutButton = document.querySelector("#logout-button");
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

let board = null;
let socket = null;
let authToken = localStorage.getItem(TOKEN_STORAGE_KEY) || "";
let authProfile = readStoredProfile();
let clientId = "";
let clientName = "";
let color = "";
let currentPosition = START_POSITION;
let moves = 0;

if (authProfile?.email) {
  loginEmailInput.value = authProfile.email;
}

syncIdentity();

loginForm.addEventListener("submit", (event) => {
  event.preventDefault();
  handleLoginSubmit().catch((error) => {
    showAuthError(loginError, error);
  });
});

registerForm.addEventListener("submit", (event) => {
  event.preventDefault();
  handleRegisterSubmit().catch((error) => {
    showAuthError(registerError, error);
  });
});

showRegisterButton.addEventListener("click", () => {
  registerEmailInput.value = loginEmailInput.value.trim();
  registerError.textContent = "";
  showPage("register");
});

showLoginButton.addEventListener("click", () => {
  loginEmailInput.value = registerEmailInput.value.trim();
  loginError.textContent = "";
  showPage("login");
});

logoutButton.addEventListener("click", () => {
  logout();
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

if (decodeJwtPayload(authToken)?.sub) {
  startGameSession().catch((error) => {
    logout();
    showAuthError(loginError, error);
  });
} else {
  clearStoredAuth();
  syncIdentity();
  showPage("login");
}

async function handleLoginSubmit() {
  loginError.textContent = "";
  const email = loginEmailInput.value.trim();
  const password = loginPasswordInput.value;

  if (!email || !password) {
    throw new Error("Email and password are required.");
  }

  const login = await authRequest("login", { email, password });
  saveAuth(login.access_token, { email });
  await startGameSession();
}

async function handleRegisterSubmit() {
  registerError.textContent = "";
  const email = registerEmailInput.value.trim();
  const username = registerUsernameInput.value.trim();
  const firstName = registerFirstNameInput.value.trim();
  const lastName = registerLastNameInput.value.trim();
  const password = registerPasswordInput.value;

  if (!email || !username || !firstName || !lastName || !password) {
    throw new Error("All fields are required.");
  }

  const registered = await authRequest("register", {
    email,
    username,
    first_name: firstName,
    last_name: lastName,
    password,
  });
  const login = await authRequest("login", { email, password });
  saveAuth(login.access_token, {
    id: registered.id,
    email: registered.email,
    username: registered.username,
  });
  await startGameSession();
}

async function startGameSession() {
  syncIdentity();
  showPage("game");
  await initializeBoard();
  connectWebSocket();
}

async function authRequest(path, body) {
  const response = await fetch(`${AUTH_URL}/${path}`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });

  if (!response.ok) {
    throw new Error(await errorMessage(response));
  }

  return response.json();
}

async function errorMessage(response) {
  try {
    const data = await response.clone().json();
    return data.message || data.error || response.statusText || "Request failed.";
  } catch {
    const text = await response.text();
    return text || response.statusText || "Request failed.";
  }
}

function saveAuth(token, profile) {
  const claims = decodeJwtPayload(token);
  if (!claims?.sub) {
    throw new Error("Login returned an invalid access token.");
  }

  authToken = token;
  authProfile = {
    ...profile,
    id: claims.sub,
    email: claims.email || profile.email,
    username: profile.username || claims.email,
  };

  localStorage.setItem(TOKEN_STORAGE_KEY, authToken);
  localStorage.setItem(PROFILE_STORAGE_KEY, JSON.stringify(authProfile));
  syncIdentity();
}

function logout() {
  if (socket) {
    socket.close();
    socket = null;
  }

  clearStoredAuth();
  syncIdentity();
  resetSessionState();
  showPage("login");
}

function clearStoredAuth() {
  authToken = "";
  authProfile = null;
  localStorage.removeItem(TOKEN_STORAGE_KEY);
  localStorage.removeItem(PROFILE_STORAGE_KEY);
}

function syncIdentity() {
  const claims = authToken ? decodeJwtPayload(authToken) : null;

  if (!claims?.sub) {
    clientId = "";
    clientName = "not signed in";
    color = "#9aa4b1";
  } else {
    clientId = claims.sub;
    clientName = authProfile?.username || claims.email || `player-${clientId.slice(0, 6)}`;
    color = colorFromId(clientId);
  }

  clientLabel.textContent = clientName;
  clientColor.style.background = color;
}

function showPage(page) {
  loginPage.classList.toggle("hidden", page !== "login");
  registerPage.classList.toggle("hidden", page !== "register");
  gamePage.classList.toggle("hidden", page !== "game");

  if (page === "game") {
    requestAnimationFrame(() => board?.resize());
  }
}

function showAuthError(target, error) {
  target.textContent = error.message || "Something went wrong.";
}

async function initializeBoard() {
  if (board) {
    board.resize();
    return;
  }

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

function connectWebSocket() {
  if (!decodeJwtPayload(authToken)?.sub) {
    logout();
    return;
  }

  if (socket) {
    socket.close();
    socket = null;
  }

  setStatus("connecting", "Connecting");

  let nextSocket;

  try {
    nextSocket = new WebSocket(websocketUrlWithToken(WS_URL, authToken));
  } catch (error) {
    setStatus("disconnected", "Connection error");
    writeLog(error.message);
    return;
  }

  nextSocket.binaryType = "arraybuffer";
  socket = nextSocket;

  nextSocket.addEventListener("open", () => {
    if (socket !== nextSocket) return;

    setStatus("connected", "Connected");
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

function resetSessionState() {
  currentPosition = START_POSITION;
  moves = 0;
  lastEvent.textContent = "none";
  moveCount.textContent = "0";
  logList.replaceChildren();
  setBoardPosition(START_POSITION, false);
  setStatus("disconnected", "Disconnected");
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

function websocketUrlWithToken(url, token) {
  const parsed = new URL(url);
  parsed.searchParams.set("token", token);
  return parsed.toString();
}

function decodeJwtPayload(token) {
  try {
    const [, payload] = token.split(".");
    if (!payload) return null;

    const normalized = payload.replace(/-/g, "+").replace(/_/g, "/");
    const padded = normalized.padEnd(
      normalized.length + ((4 - (normalized.length % 4)) % 4),
      "=",
    );
    const decoded = JSON.parse(atob(padded));

    if (decoded.exp && decoded.exp * 1000 <= Date.now()) {
      return null;
    }

    return decoded;
  } catch {
    return null;
  }
}

function readStoredProfile() {
  try {
    return JSON.parse(localStorage.getItem(PROFILE_STORAGE_KEY) || "null");
  } catch {
    return null;
  }
}

function clonePosition(position) {
  if (typeof position === "string") {
    return position;
  }

  return { ...position };
}

function isValidPosition(position) {
  return (
    position === START_POSITION ||
    (position && typeof position === "object" && !Array.isArray(position))
  );
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
    script.addEventListener("error", () => reject(new Error(`failed to load ${src}`)), {
      once: true,
    });
    document.head.append(script);
  });
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
