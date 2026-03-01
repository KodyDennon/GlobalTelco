import { env } from '$env/dynamic/public';

const PROD_API = 'https://server.globaltelco.online';
const PROD_WS = 'wss://server.globaltelco.online/ws';

const isDev =
	typeof window !== 'undefined' &&
	(window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1');

export const API_URL = env.PUBLIC_API_URL || (isDev ? 'http://localhost:3001' : PROD_API);
export const WS_URL = env.PUBLIC_WS_URL || (isDev ? 'ws://localhost:3001/ws' : PROD_WS);
