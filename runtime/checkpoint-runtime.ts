import { SharedMemory } from "./shm/shared-memory";
const VERBOSE = process.env.CHECKPOINT_VERBOSE === "1";
/*
function vlog(message: string): void {
  if (VERBOSE) console.error(message);
}*/

interface CheckpointRequest {
  type: "checkpoint";
  id: number;
  payload: {
    functionName: string;
    args: unknown[];
    context?: unknown;
    stackDepth: number;
    callerName: string | null | undefined;
  };
}

interface CheckpointCompleteMessage {
  type: "checkpoint_complete";
  id: number;
  payload: {
    functionName: string;
    durationMs: number;
    status: "ok" | "error";
    returnValue?: SerializedValue;
    error?: SerializedValue;
    memoryDeltaBytes?: number;
  };
}

interface SerializedValue {
  type: string;
  value: unknown;
  truncated: boolean;
  preview: string;
}

interface CheckpointResponse {
  id: number;
  action: "continue" | "skip" | "continue_with_args";
  returnValue?: unknown;
  args?: unknown[];
}

const responseCache = new Map<number, CheckpointResponse>();
const timedOutRequests = new Set<number>();
let isPolling = false;
let requestIdCounter = 0;
const callStack: string[] = [];
let shm: SharedMemory | null = null;

const MAX_SERIALIZED_BYTES = 32_000;
const MAX_PREVIEW_LENGTH = 80;

function getNextRequestId(): number {
  return ++requestIdCounter;
}

function detectType(value: unknown): string {
  if (value === null) return "null";
  if (value === undefined) return "undefined";
  if (Array.isArray(value)) return "Array";
  if (value instanceof Map) return "Map";
  if (value instanceof Set) return "Set";
  if (value instanceof Date) return "Date";
  if (value instanceof Error) return "Error";
  if (value instanceof Promise) return "Promise";
  if (typeof value === "function") return "Function";
  if (typeof value === "object") {
    const ctor = (value as object).constructor?.name;
    return ctor && ctor !== "Object" ? ctor : "Object";
  }
  return typeof value;
}

function buildPreview(value: unknown, type: string): string {
  try {
    if (value === null) return "null";
    if (value === undefined) return "undefined";
    if (type === "Function") return `[Function: ${(value as Function).name || "anonymous"}]`;
    if (type === "Promise") return "[Promise]";
    if (type === "Date") return (value as Date).toISOString();
    if (type === "Error") return `${(value as Error).name}: ${(value as Error).message}`;
    if (type === "Map") return `Map(${(value as Map<unknown, unknown>).size})`;
    if (type === "Set") return `Set(${(value as Set<unknown>).size})`;
    if (typeof value === "string") {
      const s = value as string;
      return s.length > MAX_PREVIEW_LENGTH ? `"${s.slice(0, MAX_PREVIEW_LENGTH)}..."` : `"${s}"`;
    }
    if (typeof value === "number" || typeof value === "boolean" || typeof value === "bigint") {
      return String(value);
    }

    const seen = new WeakSet();
    const json = JSON.stringify(value, (_k, v) => {
      if (typeof v === "bigint") return `${v}n`;
      if (typeof v === "function") return `[Function: ${v.name || "anonymous"}]`;
      if (typeof v === "object" && v !== null) {
        if (seen.has(v)) return "[Circular]";
        seen.add(v);
      }
      return v;
    });

    if (!json) return `[${type}]`;
    return json.length > MAX_PREVIEW_LENGTH ? `${json.slice(0, MAX_PREVIEW_LENGTH)}...` : json;
  } catch {
    return `[${type}]`;
  }
}

function safeSerialize(value: unknown): SerializedValue {
  const type = detectType(value);
  const preview = buildPreview(value, type);

  let serializedValue: unknown = null;
  let truncated = false;

  try {
    const seen = new WeakSet();
    const json = JSON.stringify(value, (_k, v) => {
      if (typeof v === "bigint") return `${v}n`;
      if (typeof v === "function") return `[Function: ${v.name || "anonymous"}]`;
      if (typeof v === "symbol") return v.toString();
      if (typeof v === "undefined") return "[undefined]";
      if (v instanceof Map) return { __type: "Map", entries: Array.from(v.entries()) };
      if (v instanceof Set) return { __type: "Set", values: Array.from(v.values()) };
      if (v instanceof Error)
        return {
          __type: "Error",
          name: v.name,
          message: v.message,
          stack: v.stack,
        };
      if (v instanceof Date) return { __type: "Date", iso: v.toISOString() };
      if (typeof v === "object" && v !== null) {
        if (seen.has(v)) return "[Circular]";
        seen.add(v);
      }
      return v;
    });

    if (json && json.length > MAX_SERIALIZED_BYTES) {
      serializedValue = json.slice(0, MAX_SERIALIZED_BYTES);
      truncated = true;
    } else {
      serializedValue = json ? JSON.parse(json) : preview;
    }
  } catch {
    serializedValue = preview;
  }

  return { type, value: serializedValue, truncated, preview };
}

async function pollResponses(): Promise<void> {
  if (isPolling || !shm) return;
  isPolling = true;

  try {
    while (true) {
      const response = shm.receiveCheckpointResponseJson<CheckpointResponse>();
      if (!response) break;

      if (timedOutRequests.has(response.id)) {
        timedOutRequests.delete(response.id);
        continue;
      }
      responseCache.set(response.id, response);
    }
  } catch (error) {
    console.error("[POLL] Error polling responses:", error);
  } finally {
    isPolling = false;
  }
}

export function initializeCheckpointRuntime(shmInstance: SharedMemory): void {
  shm = shmInstance;
}

export function debugQueueState(): void {
  if (!shm) {
    console.error("[DEBUG] SharedMemory not initialized");
    return;
  }
  if (VERBOSE) shm.debugQueueState();
}

async function sendCompletion(
  id: number,
  functionName: string,
  durationMs: number,
  status: "ok" | "error",
  returnValue?: SerializedValue,
  error?: SerializedValue,
  memoryDeltaBytes?: number,
): Promise<void> {
  if (!shm) return;
  const message: CheckpointCompleteMessage = {
    type: "checkpoint_complete",
    id,
    payload: {
      functionName,
      durationMs,
      status,
      returnValue,
      error,
      memoryDeltaBytes,
    },
  };

  const sendStart = performance.now();
  const sendTimeoutMs = 60_000;
  while (true) {
    try {
      shm.sendCheckpointJson(message);
      return;
    } catch (e) {
      if (
        e instanceof Error &&
        e.message === "queue full" &&
        performance.now() - sendStart < sendTimeoutMs
      ) {
        await Bun.sleep(5);
        continue;
      }
      console.error(`[CHECKPOINT] Failed to send completion for ${functionName}:`, e);
      return;
    }
  }
}
export const __checkpoint__ = {
  async execute<T>(
    functionName: string,
    fn: (...args: unknown[]) => T,
    args: unknown[],
    context?: unknown,
  ): Promise<T> {
    if (!shm) {
      throw new Error("Checkpoint runtime not initialized");
    }

    const requestId = getNextRequestId();
    const stackDepth = callStack.length;
    const callerName = stackDepth > 0 ? callStack[stackDepth - 1] : null;

    callStack.push(functionName);

    const request: CheckpointRequest = {
      type: "checkpoint",
      id: requestId,
      payload: { functionName, args, context, stackDepth, callerName },
    };

    const sendStart = performance.now();
    const sendTimeoutMs = 60_000;
    let sent = false;
    while (!sent) {
      try {
        shm.sendCheckpointJson(request);
        sent = true;
      } catch (error) {
        if (
          error instanceof Error &&
          error.message === "queue full" &&
          performance.now() - sendStart < sendTimeoutMs
        ) {
          await Bun.sleep(5);
          continue;
        }
        callStack.pop();
        console.error(`Failed to send checkpoint for ${functionName}:`, error);
        throw error;
      }
    }

    const startTime = performance.now();
    const timeoutMs = 1 * 60 * 60 * 1000;

    while (performance.now() - startTime < timeoutMs) {
      await pollResponses();

      if (responseCache.has(requestId)) {
        const response = responseCache.get(requestId)!;
        responseCache.delete(requestId);

        if (response.action === "skip") {
          const skipReturn = (response as CheckpointResponse).returnValue;
          const finalResult = skipReturn as T;

          sendCompletion(
            requestId,
            functionName,
            0,
            "ok",
            safeSerialize(finalResult),
            undefined,
            0,
          );

          callStack.pop();
          return finalResult;
        }

        if (response.action === "continue_with_args") {
          const newArgs = (response as CheckpointResponse).args ?? args;

          const memBefore = process.memoryUsage().heapUsed;
          const execStart = performance.now();
          try {
            const result = fn.apply(context, newArgs as never);
            const finalResult = result instanceof Promise ? await result : result;
            const duration = performance.now() - execStart;
            const memDelta = process.memoryUsage().heapUsed - memBefore;

            sendCompletion(
              requestId,
              functionName,
              duration,
              "ok",
              safeSerialize(finalResult),
              undefined,
              memDelta,
            );

            callStack.pop();
            return finalResult;
          } catch (err) {
            const duration = performance.now() - execStart;
            const memDelta = process.memoryUsage().heapUsed - memBefore;
            sendCompletion(
              requestId,
              functionName,
              duration,
              "error",
              undefined,
              safeSerialize(err),
              memDelta,
            );
            callStack.pop();
            throw err;
          }
        }

        if (response.action !== "continue") {
          callStack.pop();
          throw new Error(`Unknown checkpoint action: ${response.action}`);
        }

        const memBefore = process.memoryUsage().heapUsed;
        const execStart = performance.now();
        try {
          const result = fn.apply(context, args as never);
          const finalResult = result instanceof Promise ? await result : result;
          const duration = performance.now() - execStart;
          const memDelta = process.memoryUsage().heapUsed - memBefore;

          sendCompletion(
            requestId,
            functionName,
            duration,
            "ok",
            safeSerialize(finalResult),
            undefined,
            memDelta,
          );

          callStack.pop();
          return finalResult;
        } catch (err) {
          const duration = performance.now() - execStart;
          const memDelta = process.memoryUsage().heapUsed - memBefore;
          sendCompletion(
            requestId,
            functionName,
            duration,
            "error",
            undefined,
            safeSerialize(err),
            memDelta,
          );
          callStack.pop();
          throw err;
        }
      }

      await Bun.sleep(10);
    }

    callStack.pop();
    console.error(`[CHECKPOINT] Timeout for ${functionName} (id: ${requestId})`);
    timedOutRequests.add(requestId);
    responseCache.delete(requestId);
    throw new Error(`Checkpoint timeout for ${functionName} (id: ${requestId})`);
  },
};
