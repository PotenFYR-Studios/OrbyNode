/**
 * Realtime client (ADR 009): one WebSocket per tab, snapshot + events.
 *
 * Reconnect: exponential backoff with jitter (Plan §89); consumers re-hello
 * with their last seen sequence and the server resumes or forces resync.
 */

export interface Envelope {
  v: number;
  seq: number;
  stream: string;
  etype: string;
  data?: unknown;
  priority: "critical" | "droppable";
}

type Listener = (ev: Envelope) => void;
type BinaryListener = (stream: string, payload: Uint8Array) => void;

export class RealtimeClient {
  private ws: WebSocket | null = null;
  private listeners = new Map<string, Set<Listener>>();
  private binaryListeners = new Set<BinaryListener>();
  private subs = new Set<string>();
  private lastSeq = 0;
  private backoffMs = 250;
  private closed = false;

  constructor(private readonly url = resolvedUrl()) {}

  connect(): void {
    this.closed = false;
    const ws = new WebSocket(this.url);
    this.ws = ws;
    ws.binaryType = "arraybuffer";
    ws.onopen = () => {
      this.backoffMs = 250;
      ws.send(JSON.stringify({ op: "hello", last_seq: this.lastSeq || undefined }));
      for (const stream of this.subs) {
        ws.send(JSON.stringify({ op: "sub", stream }));
      }
    };
    ws.onmessage = (msg) => {
      if (msg.data instanceof ArrayBuffer) {
        const view = new DataView(msg.data);
        const slen = view.getUint32(0);
        const stream = new TextDecoder().decode(new Uint8Array(msg.data, 4, slen));
        const payload = new Uint8Array(msg.data, 4 + slen);
        for (const fn of this.binaryListeners) fn(stream, payload);
        return;
      }
      try {
        const parsed = JSON.parse(String(msg.data)) as {
          op?: string;
          seq?: number;
        } & Envelope;
        if (parsed.op) return; // op replies carry no state
        if (typeof parsed.seq === "number" && parsed.stream !== "meta") {
          this.lastSeq = Math.max(this.lastSeq, parsed.seq);
        }
        if (parsed.stream) {
          const set = this.listeners.get(parsed.stream);
          if (set) for (const fn of set) fn(parsed);
        }
      } catch {
        // ignore malformed frames; server controls the protocol
      }
    };
    ws.onclose = () => {
      if (this.closed) return;
      const jitter = Math.random() * this.backoffMs * 0.3;
      setTimeout(() => this.connect(), this.backoffMs + jitter);
      this.backoffMs = Math.min(this.backoffMs * 2, 10_000);
    };
  }

  sub(stream: string): void {
    this.subs.add(stream);
    this.ws?.send(JSON.stringify({ op: "sub", stream }));
  }

  unsub(stream: string): void {
    this.subs.delete(stream);
    this.ws?.send(JSON.stringify({ op: "unsub", stream }));
  }

  on(stream: string, fn: Listener): () => void {
    let set = this.listeners.get(stream);
    if (!set) this.listeners.set(stream, (set = new Set()));
    set.add(fn);
    return () => set!.delete(fn);
  }

  onBinary(fn: BinaryListener): () => void {
    this.binaryListeners.add(fn);
    return () => this.binaryListeners.delete(fn);
  }

  close(): void {
    this.closed = true;
    this.ws?.close();
  }
}

function resolvedUrl(): string {
  const proto = location.protocol === "https:" ? "wss:" : "ws:";
  return `${proto}//${location.host}/ws`;
}
