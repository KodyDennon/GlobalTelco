import { env } from '$env/dynamic/public';

const DEFAULT_SITE_URL = 'https://github.com/KodyDennon/GlobalTelco';

const isDev =
	typeof window !== 'undefined' &&
	(window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1');

export const SITE_URL = env.PUBLIC_SITE_URL || DEFAULT_SITE_URL;
export const API_URL = env.PUBLIC_API_URL || (isDev ? 'http://localhost:3001' : '');
export const WS_URL = env.PUBLIC_WS_URL || (isDev ? 'ws://localhost:3001/ws' : '');
