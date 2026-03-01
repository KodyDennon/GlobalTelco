const PROD_API = 'https://server.globaltelco.online';

const isDev =
	typeof window !== 'undefined' &&
	(window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1');

export const API_URL = isDev ? 'http://localhost:3001' : PROD_API;
