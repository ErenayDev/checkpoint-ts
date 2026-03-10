import { SharedMemory } from "./shm/shared-memory";

const shmId = process.env.CHECKPOINT_SHM_ID;
if (!shmId) {
  console.error("CHECKPOINT_SHM_ID not set");
  process.exit(1);
}

let shm: SharedMemory;
try {
  shm = SharedMemory.open(shmId);
} catch (error) {
  console.error(`Failed to open shared memory: ${error}`);
  process.exit(1);
}

interface ExecuteRequest {
  functionName: string;
  args: unknown[];
  context?: unknown;
}

interface ExecuteResponse {
  type: "continue" | "skip" | "error";
  returnValue?: unknown;
  error?: string;
}

export const __checkpoint__ = {
  async execute<T>(
    functionName: string,
    fn: (...args: unknown[]) => T,
    args: unknown[],
    context?: unknown,
  ): Promise<T> {
    const request: ExecuteRequest = {
      functionName,
      args,
      context: context ? context.constructor.name : undefined,
    };

    shm.writeJson({
      type: "checkpoint",
      payload: request,
    });

    let response: ExecuteResponse | null = null;
    const maxWaitMs = 30000; // 30 second timeout
    const startTime = performance.now();

    while (!response) {
      if (performance.now() - startTime > maxWaitMs) {
        throw new Error(
          `Checkpoint timeout waiting for response on ${functionName}`,
        );
      }
      response = await shm.waitAndReadJson<ExecuteResponse>(1000);
    }

    if (response.type === "error") {
      throw new Error(response.error || "Checkpoint error");
    }

    if (response.type === "skip") {
      if (response.returnValue === undefined) {
        throw new Error(
          `Checkpoint skip response missing returnValue for ${functionName}`,
        );
      }
      return response.returnValue as T;
    }

    return fn.apply(context, args as never[]);
  },
};
