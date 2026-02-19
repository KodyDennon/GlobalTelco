import { env } from '$env/dynamic/public';

export const API_URL = env.PUBLIC_API_URL || 'http://localhost:3001';
export const WS_URL = env.PUBLIC_WS_URL || 'ws://localhost:3001/ws';
