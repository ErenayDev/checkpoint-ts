import { SharedMemory } from "./shm/shared-memory";

interface CheckpointRequest {
  type: "checkpoint";
  id: number;
  payload: {
    functionName: string;
    args: unknown[];
    context?: unknown;
  };
}

interface CheckpointResponse {
  id: number;
  action: "continue" | "step_over" | "step_into";
}

const responseCache = new Map<number, CheckpointResponse>();
const timedOutRequests = new Set<number>();
let isPolling = false;
let requestIdCounter = 0;
let shm: SharedMemory | null = null;

function getNextRequestId(): number {
  return ++requestIdCounter;
}

async function pollResponses(): Promise<void> {
  if (isPolling || !shm) return;
  isPolling = true;

  try {
    let receivedCount = 0;
    while (true) {
      const response = shm.receiveCheckpointResponseJson<CheckpointResponse>();
      if (!response) break;

      if (timedOutRequests.has(response.id)) {
        console.error(
          `[POLL] Discarding late response for timed-out ID ${response.id}`,
        );
        timedOutRequests.delete(response.id);
        continue;
      }

      receivedCount++;
      console.error(
        `[POLL] Received response #${receivedCount}: id=${response.id}`,
      );
      responseCache.set(response.id, response);
    }

    if (receivedCount > 0) {
      console.error(`[POLL] Polled ${receivedCount} responses`);
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

export function debugResponseCache(): void {
  console.error("[DEBUG] ===== Response Cache =====");
  console.error(`[DEBUG] Cache size: ${responseCache.size}`);

  if (responseCache.size > 0) {
    console.error("[DEBUG] Cache contents:");
    for (const [id, response] of responseCache.entries()) {
      console.error(`[DEBUG]   ID ${id}: ${JSON.stringify(response)}`);
    }
  } else {
    console.error("[DEBUG] Cache is empty");
  }

  console.error("[DEBUG] ===========================");
}

export function debugQueueState(): void {
  if (!shm) {
    console.error("[DEBUG] SharedMemory not initialized");
    return;
  }
  shm.debugQueueState();
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

    const request: CheckpointRequest = {
      type: "checkpoint",
      id: requestId,
      payload: {
        functionName,
        args,
        context,
      },
    };

    try {
      shm.sendCheckpointJson(request);
    } catch (error) {
      console.error(`Failed to send checkpoint for ${functionName}:`, error);
      debugQueueState();
      throw error;
    }

    const startTime = performance.now();
    const timeoutMs = 30000;

    while (performance.now() - startTime < timeoutMs) {
      await pollResponses();

      if (responseCache.has(requestId)) {
        const response = responseCache.get(requestId)!;
        responseCache.delete(requestId);

        if (response.action === "continue") {
          const result = fn.apply(context, args as never);
          // if result is a Promise, await it; otherwise return directly
          return result instanceof Promise ? await result : result;
        } else if (response.action === "step_over") {
          const result = fn.apply(context, args as never);
          return result instanceof Promise ? await result : result;
        } else if (response.action === "step_into") {
          const result = fn.apply(context, args as never);
          return result instanceof Promise ? await result : result;
        } else {
          throw new Error(`Unknown checkpoint action: ${response.action}`);
        }
      }

      await Bun.sleep(10);
    }

    console.error(
      `[CHECKPOINT] Timeout for ${functionName} (id: ${requestId})`,
    );
    timedOutRequests.add(requestId);
    responseCache.delete(requestId);
    debugResponseCache();
    debugQueueState();
    throw new Error(
      `Checkpoint timeout for ${functionName} (id: ${requestId})`,
    );
  },
};
