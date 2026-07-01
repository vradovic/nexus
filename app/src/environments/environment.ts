const API_BASE = 'http://localhost';

// TODO: Add reverse proxy
export const environment = {
    authApiUrl: `${API_BASE}:3001`,
    socialApiUrl: `${API_BASE}:3002`,
    matchmakingApiUrl: `${API_BASE}:3003`,
    realtimeApiUrl: `${API_BASE}:3000`,
    realtimeWsUrl: 'ws://localhost:3000',
};
