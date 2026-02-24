import { SharedMemory } from "./shm/shared-memory";

const shmId = process.env.CHECKPOINT_SHM_ID;
if (!shmId) {
  console.error("CHECKPOINT_SHM_ID not set");
  process.exit(1);
}

const shm = SharedMemory.open(shmId);

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
  execute<T>(
    functionName: string,
    fn: (...args: unknown[]) => T,
    args: unknown[],
    context?: unknown,
  ): T {
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

    while (!response) {
      response = shm.waitAndReadJson<ExecuteResponse>(1000);
    }

    if (response.type === "error") {
      throw new Error(response.error || "Checkpoint error");
    }

    if (response.type === "skip") {
      return response.returnValue as T;
    }

    return fn.apply(context, args as never[]);
  },
};
