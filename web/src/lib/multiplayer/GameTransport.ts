/**
 * GameTransport — abstraction layer for multiplayer transport.
 * Supports WebTransport (preferred) with WebSocket fallback.
 *
 * Channels:
 *   - Reliable: commands, command acks, chat (ordered, guaranteed delivery)
 *   - Unreliable: tick updates, delta broadcasts (latest-wins, loss tolerable)
 */

import { encode, decode } from '@msgpack/msgpack';

export interface GameTransport {
	connect(url: string): Promise<void>;
	sendReliable(data: Uint8Array): void;
	sendUnreliable(data: Uint8Array): void;
	onReliable(handler: (data: Uint8Array) => void): void;
	onUnreliable(handler: (data: Uint8Array) => void): void;
	onClose(handler: (reason: string) => void): void;
	close(): void;
	isConnected(): boolean;
	readonly transportType: 'webtransport' | 'websocket';
}

// ── WebTransport Implementation ──────────────────────────────────────────────

export class WebTransportTransport implements GameTransport {
	readonly transportType = 'webtransport' as const;
	private transport: WebTransport | null = null;
	private reliableStream: WritableStreamDefaultWriter<Uint8Array> | null = null;
	private reliableReader: ReadableStreamDefaultReader<Uint8Array> | null = null;
	private reliableHandler: ((data: Uint8Array) => void) | null = null;
	private unreliableHandler: ((data: Uint8Array) => void) | null = null;
	private closeHandler: ((reason: string) => void) | null = null;
	private connected = false;

	async connect(url: string): Promise<void> {
		// Convert wss:// to https:// for WebTransport
		const wtUrl = url.replace(/^wss?:\/\//, 'https://');
		this.transport = new WebTransport(wtUrl);

		await this.transport.ready;
		this.connected = true;

		// Set up reliable bidirectional stream (Stream 0)
		const bidiStream = await this.transport.createBidirectionalStream();
		this.reliableStream = bidiStream.writable.getWriter();

		// Read reliable messages in background
		this.reliableReader = bidiStream.readable.getReader();
		this.readReliableLoop();

		// Read unreliable datagrams in background
		this.readDatagramLoop();

		// Monitor connection closed
		this.transport.closed.then(() => {
			this.connected = false;
			this.closeHandler?.('transport closed');
		}).catch((e) => {
			this.connected = false;
			this.closeHandler?.(e?.message ?? 'transport error');
		});
	}

	private async readReliableLoop(): Promise<void> {
		if (!this.reliableReader) return;
		try {
			while (true) {
				const { value, done } = await this.reliableReader.read();
				if (done) break;
				if (value && this.reliableHandler) {
					this.reliableHandler(value);
				}
			}
		} catch {
			// Stream closed
		}
	}

	private async readDatagramLoop(): Promise<void> {
		if (!this.transport) return;
		const reader = this.transport.datagrams.readable.getReader();
		try {
			while (true) {
				const { value, done } = await reader.read();
				if (done) break;
				if (value && this.unreliableHandler) {
					this.unreliableHandler(value);
				}
			}
		} catch {
			// Datagram stream closed
		}
	}

	sendReliable(data: Uint8Array): void {
		this.reliableStream?.write(data).catch(() => {});
	}

	sendUnreliable(data: Uint8Array): void {
		if (!this.transport) return;
		const writer = this.transport.datagrams.writable.getWriter();
		writer.write(data).catch(() => {});
		writer.releaseLock();
	}

	onReliable(handler: (data: Uint8Array) => void): void {
		this.reliableHandler = handler;
	}

	onUnreliable(handler: (data: Uint8Array) => void): void {
		this.unreliableHandler = handler;
	}

	onClose(handler: (reason: string) => void): void {
		this.closeHandler = handler;
	}

	isConnected(): boolean {
		return this.connected;
	}

	close(): void {
		this.connected = false;
		try { this.reliableStream?.close(); } catch {}
		try { this.reliableReader?.cancel(); } catch {}
		try { this.transport?.close(); } catch {}
		this.transport = null;
		this.reliableStream = null;
		this.reliableReader = null;
	}
}

// ── WebSocket Fallback Implementation ────────────────────────────────────────

export class WebSocketTransport implements GameTransport {
	readonly transportType = 'websocket' as const;
	private ws: WebSocket | null = null;
	private reliableHandler: ((data: Uint8Array) => void) | null = null;
	private unreliableHandler: ((data: Uint8Array) => void) | null = null;
	private closeHandler: ((reason: string) => void) | null = null;
	private connected = false;

	async connect(url: string): Promise<void> {
		return new Promise((resolve, reject) => {
			this.ws = new WebSocket(url);
			this.ws.binaryType = 'arraybuffer';

			this.ws.onopen = () => {
				this.connected = true;
				resolve();
			};

			this.ws.onerror = (e) => {
				if (!this.connected) reject(new Error('WebSocket connection failed'));
			};

			this.ws.onmessage = (event) => {
				if (event.data instanceof ArrayBuffer) {
					const data = new Uint8Array(event.data);
					// WebSocket is inherently reliable — route all messages
					// through the reliable handler only. The application layer
					// handles message type routing via its own dispatch.
					this.reliableHandler?.(data);
				}
			};

			this.ws.onclose = (event) => {
				this.connected = false;
				this.closeHandler?.(event.reason || `code ${event.code}`);
			};
		});
	}

	sendReliable(data: Uint8Array): void {
		if (this.ws?.readyState === WebSocket.OPEN) {
			this.ws.send(data);
		}
	}

	// WebSocket doesn't support unreliable — falls back to reliable
	sendUnreliable(data: Uint8Array): void {
		this.sendReliable(data);
	}

	onReliable(handler: (data: Uint8Array) => void): void {
		this.reliableHandler = handler;
	}

	onUnreliable(handler: (data: Uint8Array) => void): void {
		this.unreliableHandler = handler;
	}

	onClose(handler: (reason: string) => void): void {
		this.closeHandler = handler;
	}

	isConnected(): boolean {
		return this.connected;
	}

	close(): void {
		this.connected = false;
		if (this.ws) {
			this.ws.close(1000, 'Client disconnect');
			this.ws = null;
		}
	}
}

// ── Factory ──────────────────────────────────────────────────────────────────

/** Feature-detect and create the best available transport. */
export function createTransport(_url: string): GameTransport {
	if (typeof WebTransport !== 'undefined') {
		return new WebTransportTransport();
	}
	return new WebSocketTransport();
}

/** Encode a message object to binary (MessagePack). */
export function encodeMessage(msg: Record<string, unknown>): Uint8Array {
	return encode(msg) as Uint8Array;
}

/** Decode a binary message (MessagePack) to an object. */
export function decodeMessage(data: Uint8Array): Record<string, unknown> {
	return decode(data) as Record<string, unknown>;
}
