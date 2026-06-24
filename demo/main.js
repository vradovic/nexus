import $ from "jquery";
import "@chrisoakman/chessboardjs/dist/chessboard-1.0.0.css";
import chessboardScriptUrl from "@chrisoakman/chessboardjs/dist/chessboard-1.0.0.js?url";
import "./styles.css";

window.$ = $;
window.jQuery = $;

const START_POSITION = "start";
const AUTH_URL = "http://127.0.0.1:3001";
const SOCIAL_URL = "http://127.0.0.1:3002";
const WS_URL = "ws://127.0.0.1:3000/ws";
const MATCHMAKING_URL = "http://127.0.0.1:3003";
const TOKEN_STORAGE_KEY = "nexus-demo-access-token";
const PROFILE_STORAGE_KEY = "nexus-demo-profile";
const PIECE_THEME_URL = "https://chessboardjs.com/img/chesspieces/wikipedia/{piece}.png";
const MATCHMAKING_POLL_INTERVAL_MS = 1500;
const MATCHMAKING_QUEUES = {
  duel: {
    label: "Duel",
    requiredPlayers: 2,
  },
};

const loginPage = document.querySelector("#login-page");
const registerPage = document.querySelector("#register-page");
const lobbyPage = document.querySelector("#lobby-page");
const friendsPage = document.querySelector("#friends-page");
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

const logoutButtons = document.querySelectorAll("[data-logout-button]");
const clientLabels = document.querySelectorAll("[data-client-label]");
const clientColors = document.querySelectorAll("[data-client-color]");
const statusDots = document.querySelectorAll("[data-status-dot]");
const statusTexts = document.querySelectorAll("[data-status-text]");
const showFriendsButton = document.querySelector("#show-friends-button");
const showLobbyButton = document.querySelector("#show-lobby-button");
const refreshFriendsButton = document.querySelector("#refresh-friends-button");
const friendsStatus = document.querySelector("#friends-status");
const friendsList = document.querySelector("#friends-list");
const incomingFriendRequestsList = document.querySelector("#incoming-friend-requests");
const outgoingFriendRequestsList = document.querySelector("#outgoing-friend-requests");
const blockedUsersList = document.querySelector("#blocked-users-list");

const queueButtons = document.querySelectorAll("[data-ticket-key]");
const queueStatus = document.querySelector("#queue-status");
const queueLabel = document.querySelector("#queue-label");
const ticketLabel = document.querySelector("#ticket-label");
const matchLabel = document.querySelector("#match-label");
const matchExpiresLabel = document.querySelector("#match-expires-label");
const leaveQueueButton = document.querySelector("#leave-queue-button");

const resetButton = document.querySelector("#reset-button");
const flipButton = document.querySelector("#flip-button");
const logList = document.querySelector("#log");
const positionLabel = document.querySelector("#position-label");
const channelLabel = document.querySelector("#channel-label");
const lastEvent = document.querySelector("#last-event");
const moveCount = document.querySelector("#move-count");
const opponentLabel = document.querySelector("#opponent-label");
const sendFriendRequestButton = document.querySelector("#send-friend-request-button");
const friendRequestStatus = document.querySelector("#friend-request-status");
const blockOpponentButton = document.querySelector("#block-opponent-button");
const blockStatus = document.querySelector("#block-status");

let board = null;
let socket = null;
let authToken = localStorage.getItem(TOKEN_STORAGE_KEY) || "";
let authProfile = readStoredProfile();
let clientId = "";
let clientName = "";
let color = "";
let currentPage = "login";
let currentPosition = START_POSITION;
let moves = 0;
let queueBusy = false;
let queuedTicket = null;
let pendingMatch = null;
let awaitingChannelMatchId = "";
let awaitingChannelTicketKey = "";
let confirmingMatchId = "";
let activeMatch = null;
let matchmakingPollTimer = null;
let statusRequestInFlight = false;
let friendRequestBusy = false;
let blockBusy = false;
let friendsRequestInFlight = false;
const confirmedMatchIds = new Set();
const sentFriendRequestRecipientIds = new Set();
const blockedUserIds = new Set();

if (authProfile?.email) {
  loginEmailInput.value = authProfile.email;
}

syncIdentity();
renderLobbyState();

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

logoutButtons.forEach((button) => {
  button.addEventListener("click", () => {
    logout();
  });
});

queueButtons.forEach((button) => {
  button.addEventListener("click", () => {
    joinQueue(button.dataset.ticketKey).catch((error) => {
      renderLobbyState();
      queueStatus.textContent = error.message || "Failed to join queue.";
    });
  });
});

leaveQueueButton.addEventListener("click", () => {
  leaveQueue().catch((error) => {
    renderLobbyState();
    queueStatus.textContent = error.message || "Failed to leave queue.";
  });
});

showFriendsButton.addEventListener("click", () => {
  openFriendsPage().catch((error) => {
    friendsStatus.textContent = error.message || "Failed to load friends.";
  });
});

showLobbyButton.addEventListener("click", () => {
  showPage("lobby");
  refreshMatchmakingStatus().catch((error) => {
    queueStatus.textContent = error.message || "Failed to refresh matchmaking.";
  });
  startMatchmakingPolling();
});

refreshFriendsButton.addEventListener("click", () => {
  loadFriendsPage().catch((error) => {
    friendsStatus.textContent = error.message || "Failed to load friends.";
  });
});

incomingFriendRequestsList.addEventListener("click", (event) => {
  const button = event.target.closest("button[data-friend-request-action]");
  if (!button) return;

  handleFriendRequestAction(button.dataset.friendRequestAction, button.dataset.requestId).catch(
    (error) => {
      friendsStatus.textContent = error.message || "Failed to update friend request.";
    },
  );
});

blockedUsersList.addEventListener("click", (event) => {
  const button = event.target.closest("button[data-unblock-user-id]");
  if (!button) return;

  unblockUser(button.dataset.unblockUserId).catch((error) => {
    friendsStatus.textContent = error.message || "Failed to unblock user.";
  });
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

sendFriendRequestButton.addEventListener("click", () => {
  sendFriendRequestToOpponent().catch((error) => {
    friendRequestBusy = false;
    renderGameSocialState();
    friendRequestStatus.textContent = error.message || "Failed to send friend request.";
  });
});

blockOpponentButton.addEventListener("click", () => {
  blockOpponent().catch((error) => {
    blockBusy = false;
    renderGameSocialState();
    blockStatus.textContent = error.message || "Failed to block player.";
  });
});

window.addEventListener("resize", () => {
  board?.resize();
});

if (decodeJwtPayload(authToken)?.sub) {
  startLobbySession().catch((error) => {
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
  await startLobbySession();
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
  await startLobbySession();
}

async function startLobbySession() {
  syncIdentity();
  activeMatch = null;
  channelLabel.textContent = "none";
  showPage("lobby");
  ensureWebSocketConnected();
  await refreshBlockedUsers().catch(() => {});
  await refreshMatchmakingStatus().catch((error) => {
    queueStatus.textContent = error.message || "Failed to refresh matchmaking.";
  });
  startMatchmakingPolling();
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

async function matchmakingRequest(path, options = {}) {
  const { body, method = "GET" } = options;
  const headers = {
    authorization: `Bearer ${authToken}`,
  };

  if (body !== undefined) {
    headers["content-type"] = "application/json";
  }

  const response = await fetch(`${MATCHMAKING_URL}/${path}`, {
    method,
    headers,
    body: body === undefined ? undefined : JSON.stringify(body),
  });

  if (response.status === 401) {
    logout();
    throw new Error("Session expired. Sign in again.");
  }

  if (!response.ok) {
    throw new Error(await errorMessage(response));
  }

  if (response.status === 204) {
    return null;
  }

  return response.json();
}

async function socialRequest(path, options = {}) {
  const { body, method = "GET" } = options;
  const headers = {
    authorization: `Bearer ${authToken}`,
  };

  if (body !== undefined) {
    headers["content-type"] = "application/json";
  }

  const response = await fetch(`${SOCIAL_URL}/${path}`, {
    method,
    headers,
    body: body === undefined ? undefined : JSON.stringify(body),
  });

  if (response.status === 401) {
    logout();
    throw new Error("Session expired. Sign in again.");
  }

  if (!response.ok) {
    throw new Error(await errorMessage(response));
  }

  if (response.status === 204) {
    return null;
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
  stopMatchmakingPolling();

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

  clientLabels.forEach((label) => {
    label.textContent = clientName;
  });
  clientColors.forEach((swatch) => {
    swatch.style.background = color;
  });
}

function showPage(page) {
  currentPage = page;
  loginPage.classList.toggle("hidden", page !== "login");
  registerPage.classList.toggle("hidden", page !== "register");
  lobbyPage.classList.toggle("hidden", page !== "lobby");
  friendsPage.classList.toggle("hidden", page !== "friends");
  gamePage.classList.toggle("hidden", page !== "game");

  if (page === "game") {
    requestAnimationFrame(() => board?.resize());
  }
}

function showAuthError(target, error) {
  target.textContent = error.message || "Something went wrong.";
}

async function joinQueue(ticketKey) {
  if (!ticketKey || queueBusy || queuedTicket || pendingMatch || awaitingChannelMatchId) {
    return;
  }

  queueBusy = true;
  queueStatus.textContent = `Joining ${formatQueueName(ticketKey)}`;
  renderLobbyState();
  ensureWebSocketConnected();

  try {
    queuedTicket = await matchmakingRequest("join", {
      method: "POST",
      body: { ticket_key: ticketKey },
    });
    pendingMatch = null;
    awaitingChannelMatchId = "";
    awaitingChannelTicketKey = "";
    startMatchmakingPolling();
  } catch (error) {
    if (error.message.includes("already in queue") || error.message.includes("pending match")) {
      await refreshMatchmakingStatus();
    }
    throw error;
  } finally {
    queueBusy = false;
    renderLobbyState();
  }
}

async function leaveQueue() {
  if (queueBusy || !queuedTicket || pendingMatch || awaitingChannelMatchId) {
    return;
  }

  queueBusy = true;
  queueStatus.textContent = "Leaving queue";
  renderLobbyState();

  try {
    await matchmakingRequest("leave", { method: "POST" });
    queuedTicket = null;
    pendingMatch = null;
    awaitingChannelMatchId = "";
    awaitingChannelTicketKey = "";
    confirmedMatchIds.clear();
  } finally {
    queueBusy = false;
    renderLobbyState();
  }
}

function startMatchmakingPolling() {
  if (matchmakingPollTimer) {
    return;
  }

  matchmakingPollTimer = window.setInterval(() => {
    refreshMatchmakingStatus().catch((error) => {
      queueStatus.textContent = error.message || "Failed to refresh matchmaking.";
    });
  }, MATCHMAKING_POLL_INTERVAL_MS);
}

function stopMatchmakingPolling() {
  if (!matchmakingPollTimer) {
    return;
  }

  window.clearInterval(matchmakingPollTimer);
  matchmakingPollTimer = null;
}

async function refreshMatchmakingStatus() {
  if (!decodeJwtPayload(authToken)?.sub || currentPage !== "lobby" || statusRequestInFlight) {
    return;
  }

  statusRequestInFlight = true;

  try {
    const status = await matchmakingRequest("status");
    queuedTicket = status.ticket || null;
    pendingMatch = status.pending_match || null;
    renderLobbyState();

    if (pendingMatch) {
      await confirmPendingMatch(pendingMatch);
    }
  } finally {
    statusRequestInFlight = false;
  }
}

async function openFriendsPage() {
  stopMatchmakingPolling();
  showPage("friends");
  await loadFriendsPage();
}

async function loadFriendsPage() {
  if (friendsRequestInFlight) {
    return;
  }

  friendsRequestInFlight = true;
  friendsStatus.textContent = "Loading";

  try {
    const [friends, requests, blockedUsers] = await Promise.all([
      socialRequest("friends"),
      socialRequest("friend-requests"),
      socialRequest("blocks"),
    ]);
    renderFriendsPage(
      friends || [],
      requests || { incoming: [], outgoing: [] },
      blockedUsers || [],
    );
    friendsStatus.textContent = "Ready";
  } finally {
    friendsRequestInFlight = false;
  }
}

async function refreshBlockedUsers() {
  const blockedUsers = await socialRequest("blocks");
  syncBlockedUsers(blockedUsers || []);
  renderGameSocialState();
}

async function handleFriendRequestAction(action, requestId) {
  if (!requestId || (action !== "accept" && action !== "decline")) {
    return;
  }

  friendsStatus.textContent = "Updating";
  await socialRequest(`friend-requests/${requestId}/${action}`, { method: "POST" });
  await loadFriendsPage();
}

function renderFriendsPage(friends, requests, blockedUsers) {
  sentFriendRequestRecipientIds.clear();
  for (const request of requests.outgoing || []) {
    sentFriendRequestRecipientIds.add(request.recipient_id);
  }

  syncBlockedUsers(blockedUsers || []);

  friendsList.replaceChildren(
    ...listItemsOrEmpty(
      friends,
      (friend) => socialListItem(profileName(friend.first_name, friend.last_name, friend.friend_id)),
      "No friends yet",
    ),
  );

  incomingFriendRequestsList.replaceChildren(
    ...listItemsOrEmpty(
      requests.incoming || [],
      (request) =>
        socialRequestItem({
          label: profileName(
            request.requester_first_name,
            request.requester_last_name,
            request.requester_id,
          ),
          requestId: request.id,
          actions: true,
        }),
      "No incoming requests",
    ),
  );

  outgoingFriendRequestsList.replaceChildren(
    ...listItemsOrEmpty(
      requests.outgoing || [],
      (request) =>
        socialRequestItem({
          label: profileName(
            request.recipient_first_name,
            request.recipient_last_name,
            request.recipient_id,
          ),
          requestId: request.id,
          actions: false,
        }),
      "No sent requests",
    ),
  );

  blockedUsersList.replaceChildren(
    ...listItemsOrEmpty(
      blockedUsers || [],
      (blockedUser) =>
        blockedUserItem(
          profileName(
            blockedUser.first_name,
            blockedUser.last_name,
            blockedUser.blocked_user_id,
          ),
          blockedUser.blocked_user_id,
        ),
      "No blocked users",
    ),
  );

  renderGameSocialState();
}

function syncBlockedUsers(blockedUsers) {
  blockedUserIds.clear();
  for (const blockedUser of blockedUsers || []) {
    blockedUserIds.add(blockedUser.blocked_user_id);
  }
}

async function confirmPendingMatch(match) {
  if (
    confirmingMatchId === match.id ||
    confirmedMatchIds.has(match.id) ||
    awaitingChannelMatchId === match.id
  ) {
    return;
  }

  if (!isSocketOpen()) {
    ensureWebSocketConnected();
    queueStatus.textContent = "Match found. Connecting";
    return;
  }

  confirmingMatchId = match.id;
  queueStatus.textContent = "Match found. Confirming";

  try {
    await matchmakingRequest(`matches/${match.id}/confirm`, { method: "POST" });
    confirmedMatchIds.add(match.id);
    awaitingChannelMatchId = match.id;
    awaitingChannelTicketKey = match.ticket_key;
    queueStatus.textContent = "Match confirmed. Opening channel";
  } catch (error) {
    if (error.message.includes("timed out") || error.message.includes("not found")) {
      pendingMatch = null;
      awaitingChannelMatchId = "";
      awaitingChannelTicketKey = "";
      confirmedMatchIds.delete(match.id);
    }
    throw error;
  } finally {
    confirmingMatchId = "";
    renderLobbyState();
  }
}

function renderLobbyState() {
  const activeQueueKey =
    pendingMatch?.ticket_key || queuedTicket?.ticket_key || awaitingChannelTicketKey;
  const hasQueueState = Boolean(queuedTicket || pendingMatch || awaitingChannelMatchId);

  queueLabel.textContent = activeQueueKey ? formatQueueName(activeQueueKey) : "none";
  ticketLabel.textContent = queuedTicket ? shortId(queuedTicket.id) : "none";
  matchLabel.textContent = pendingMatch
    ? shortId(pendingMatch.id)
    : awaitingChannelMatchId
      ? shortId(awaitingChannelMatchId)
      : "none";
  matchExpiresLabel.textContent = pendingMatch?.expires_at_unix_seconds
    ? secondsUntil(pendingMatch.expires_at_unix_seconds)
    : "none";

  if (queueBusy) {
    queueStatus.textContent ||= "Working";
  } else if (pendingMatch && awaitingChannelMatchId === pendingMatch.id) {
    queueStatus.textContent = "Match confirmed. Opening channel";
  } else if (pendingMatch) {
    queueStatus.textContent = isSocketOpen()
      ? "Match found. Confirming"
      : "Match found. Connecting";
  } else if (awaitingChannelMatchId) {
    queueStatus.textContent = "Match confirmed. Opening channel";
  } else if (queuedTicket) {
    queueStatus.textContent = `Searching ${formatQueueName(queuedTicket.ticket_key)}`;
  } else {
    queueStatus.textContent = "Select a queue";
  }

  queueButtons.forEach((button) => {
    const isActive = button.dataset.ticketKey === activeQueueKey;
    button.classList.toggle("active", isActive);
    button.disabled = queueBusy || hasQueueState;
  });

  leaveQueueButton.disabled =
    queueBusy || !queuedTicket || Boolean(pendingMatch || awaitingChannelMatchId);
}

function formatQueueName(ticketKey) {
  return MATCHMAKING_QUEUES[ticketKey]?.label || ticketKey;
}

function shortId(id) {
  return id ? id.slice(0, 8) : "none";
}

function profileName(firstName, lastName, fallbackId) {
  const name = [firstName, lastName].filter(Boolean).join(" ").trim();
  return name || shortId(fallbackId);
}

function listItemsOrEmpty(items, renderItem, emptyText) {
  if (!items.length) {
    const emptyItem = document.createElement("li");
    emptyItem.className = "empty";
    emptyItem.textContent = emptyText;
    return [emptyItem];
  }

  return items.map(renderItem);
}

function socialListItem(label) {
  const item = document.createElement("li");
  const name = document.createElement("span");
  name.textContent = label;
  item.append(name);
  return item;
}

function socialRequestItem({ label, requestId, actions }) {
  const item = socialListItem(label);
  if (!actions) {
    return item;
  }

  const actionGroup = document.createElement("div");
  actionGroup.className = "social-list-actions";

  const acceptButton = document.createElement("button");
  acceptButton.type = "button";
  acceptButton.dataset.friendRequestAction = "accept";
  acceptButton.dataset.requestId = requestId;
  acceptButton.textContent = "Accept";

  const declineButton = document.createElement("button");
  declineButton.type = "button";
  declineButton.dataset.friendRequestAction = "decline";
  declineButton.dataset.requestId = requestId;
  declineButton.textContent = "Decline";

  actionGroup.append(acceptButton, declineButton);
  item.append(actionGroup);
  return item;
}

function blockedUserItem(label, blockedUserId) {
  const item = socialListItem(label);
  const actionGroup = document.createElement("div");
  actionGroup.className = "social-list-actions";

  const unblockButton = document.createElement("button");
  unblockButton.type = "button";
  unblockButton.dataset.unblockUserId = blockedUserId;
  unblockButton.textContent = "Unblock";

  actionGroup.append(unblockButton);
  item.append(actionGroup);
  return item;
}

function secondsUntil(unixSeconds) {
  const seconds = Math.max(0, unixSeconds - Math.floor(Date.now() / 1000));
  return `${seconds}s`;
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

  if (!activeMatch) {
    writeLog("join a match before moving pieces");
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

function ensureWebSocketConnected() {
  if (socket?.readyState === WebSocket.OPEN || socket?.readyState === WebSocket.CONNECTING) {
    return;
  }

  connectWebSocket();
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
    if (currentPage === "game") {
      writeLog(`connected as ${clientName}`);
    }
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
    if (currentPage === "game") {
      writeLog("socket closed");
    }
  });

  nextSocket.addEventListener("error", () => {
    if (socket !== nextSocket) return;

    setStatus("disconnected", "Connection error");
    if (currentPage === "game") {
      writeLog("socket error");
    }
  });
}

function sendBoardPosition({ action, move, position = currentPosition }) {
  return sendPayload({
    type: "chess.position",
    action,
    clientId,
    clientName,
    color,
    matchId: activeMatch?.matchId || null,
    channel: activeMatch?.channel || null,
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

  if (payload.type === "match.found") {
    enterGameChannel(payload).catch((error) => {
      writeLog(error.message || "failed to enter match channel");
    });
    return;
  }

  if (!activeMatch) {
    return;
  }

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
    if (payload.match_over) {
      handleMatchOver(payload);
    }
    return;
  }

  writeLog(`received ${payload.type || "unknown"} payload`);
}

async function enterGameChannel(payload) {
  const nextMatch = normalizeMatchPayload(payload);

  if (!nextMatch.channel || !nextMatch.matchId) {
    throw new Error("match notification was missing channel details");
  }

  if (activeMatch?.matchId === nextMatch.matchId && currentPage === "game") {
    return;
  }

  activeMatch = nextMatch;
  queuedTicket = null;
  pendingMatch = null;
  awaitingChannelMatchId = "";
  awaitingChannelTicketKey = "";
  confirmedMatchIds.delete(nextMatch.matchId);
  stopMatchmakingPolling();
  renderLobbyState();

  resetBoardState();
  channelLabel.textContent = nextMatch.channel;
  await refreshBlockedUsers().catch(() => {});
  renderGameSocialState();
  showPage("game");
  await initializeBoard();
  board?.resize();
  writeLog(`match found: ${formatQueueName(nextMatch.ticketKey)}`);

  sendPayload({
    type: "chess.sync_request",
    clientId,
    clientName,
    color,
    matchId: nextMatch.matchId,
    channel: nextMatch.channel,
    createdAt: Date.now(),
  });
}

function handleMatchOver(payload) {
  const summary = matchOverSummary(payload);
  const finishedMatchId = payload.matchId || payload.match_id || activeMatch?.matchId || "";

  activeMatch = null;
  queuedTicket = null;
  pendingMatch = null;
  awaitingChannelMatchId = "";
  awaitingChannelTicketKey = "";
  confirmingMatchId = "";
  if (finishedMatchId) {
    confirmedMatchIds.delete(finishedMatchId);
  }

  resetBoardState();
  showPage("lobby");
  renderLobbyState();
  queueStatus.textContent = summary;
  startMatchmakingPolling();
}

function normalizeMatchPayload(payload) {
  return {
    type: payload.type,
    matchId: payload.matchId || payload.match_id || "",
    channel: payload.channel || "",
    ticketKey: payload.ticketKey || payload.ticket_key || "",
    playerIds: payload.playerIds || payload.player_ids || [],
  };
}

function resetSessionState() {
  queuedTicket = null;
  pendingMatch = null;
  awaitingChannelMatchId = "";
  awaitingChannelTicketKey = "";
  confirmingMatchId = "";
  activeMatch = null;
  queueBusy = false;
  statusRequestInFlight = false;
  confirmedMatchIds.clear();
  renderLobbyState();
  resetBoardState();
  setStatus("disconnected", "Disconnected");
}

function resetBoardState() {
  currentPosition = START_POSITION;
  moves = 0;
  channelLabel.textContent = "none";
  lastEvent.textContent = "none";
  moveCount.textContent = "0";
  logList.replaceChildren();
  renderGameSocialState();
  setBoardPosition(START_POSITION, false);
}

async function sendFriendRequestToOpponent() {
  const recipientId = opponentId();
  if (
    !recipientId ||
    friendRequestBusy ||
    sentFriendRequestRecipientIds.has(recipientId) ||
    blockedUserIds.has(recipientId)
  ) {
    return;
  }

  friendRequestBusy = true;
  renderGameSocialState();
  friendRequestStatus.textContent = "Sending";

  try {
    await socialRequest("friend-requests", {
      method: "POST",
      body: { recipient_id: recipientId },
    });
    sentFriendRequestRecipientIds.add(recipientId);
    friendRequestStatus.textContent = "Friend request sent";
  } finally {
    friendRequestBusy = false;
    renderGameSocialState();
  }
}

async function blockOpponent() {
  const blockedUserId = opponentId();
  if (!blockedUserId || blockBusy || blockedUserIds.has(blockedUserId)) {
    return;
  }

  blockBusy = true;
  renderGameSocialState();
  blockStatus.textContent = "Blocking";

  try {
    const blockedUser = await socialRequest("blocks", {
      method: "POST",
      body: { blocked_user_id: blockedUserId },
    });
    blockedUserIds.add(blockedUser?.blocked_user_id || blockedUserId);
    blockStatus.textContent = "Player blocked";
    friendRequestStatus.textContent = "";
    if (currentPage === "friends") {
      await loadFriendsPage();
    }
  } finally {
    blockBusy = false;
    renderGameSocialState();
  }
}

async function unblockUser(blockedUserId) {
  if (!blockedUserId) {
    return;
  }

  friendsStatus.textContent = "Updating";
  await socialRequest(`blocks/${blockedUserId}`, { method: "DELETE" });
  blockedUserIds.delete(blockedUserId);
  await loadFriendsPage();
  renderGameSocialState();
}

function renderGameSocialState() {
  if (
    !opponentLabel ||
    !sendFriendRequestButton ||
    !friendRequestStatus ||
    !blockOpponentButton ||
    !blockStatus
  ) {
    return;
  }

  const recipientId = opponentId();
  opponentLabel.textContent = recipientId ? shortId(recipientId) : "none";

  if (!recipientId) {
    sendFriendRequestButton.disabled = true;
    friendRequestStatus.textContent = "";
    blockOpponentButton.disabled = true;
    blockStatus.textContent = "";
    return;
  }

  if (blockedUserIds.has(recipientId)) {
    sendFriendRequestButton.disabled = true;
    friendRequestStatus.textContent = "";
    blockOpponentButton.disabled = true;
    blockStatus.textContent ||= "Player blocked";
    return;
  }

  if (sentFriendRequestRecipientIds.has(recipientId)) {
    sendFriendRequestButton.disabled = true;
    friendRequestStatus.textContent ||= "Friend request sent";
  } else {
    sendFriendRequestButton.disabled = friendRequestBusy;
    if (!friendRequestBusy) {
      friendRequestStatus.textContent = "";
    }
  }

  blockOpponentButton.disabled = blockBusy;
  if (!blockBusy) {
    blockStatus.textContent = "";
  }
}

function opponentId() {
  if (!activeMatch?.playerIds?.length || !clientId) {
    return "";
  }

  return activeMatch.playerIds.find((playerId) => playerId !== clientId) || "";
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
  if (payload.match_over) {
    writeLog(matchOverSummary(payload));
    return;
  }

  if (payload.action === "rejected") {
    writeLog(payload.error || "move rejected");
    return;
  }

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

function matchOverSummary(payload) {
  if (payload.end_reason === "checkmate") {
    return `Checkmate. ${winnerLabel(payload.winner)} wins.`;
  }

  if (payload.end_reason === "stalemate") {
    return "Stalemate. Match over.";
  }

  return "Match over.";
}

function winnerLabel(winner) {
  if (winner === "w") return "White";
  if (winner === "b") return "Black";
  return "No one";
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
  statusDots.forEach((dot) => {
    dot.className = `status-dot ${kind}`;
  });
  statusTexts.forEach((label) => {
    label.textContent = text;
  });
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
