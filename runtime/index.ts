import { SharedMemory } from "./shm/shared-memory";

const shmId = process.env.CHECKPOINT_SHM_ID;
if (!shmId) {
  console.error("CHECKPOINT_SHM_ID not set");
  process.exit(1);
}

let shm: SharedMemory;
try {
  shm = SharedMemory.open(shmId);
  console.error(`[DEBUG] SHM opened successfully`);
} catch (error) {
  console.error(`[DEBUG] Failed to open SHM: ${error}`);
  process.exit(1);
}

interface Message {
  type: string;
  payload?: unknown;
}

interface CheckpointPayload {
  functionName: string;
  args: unknown[];
  context?: string;
}

const appPath = process.env.CHECKPOINT_APP_PATH;

function handleCheckpoint(payload: CheckpointPayload): void {
  const { functionName, args, context } = payload;
  const logMessage = context
    ? `${context}.${functionName}(${JSON.stringify(args)})`
    : `${functionName}(${JSON.stringify(args)})`;

  shm.writeJson({
    log: logMessage,
    current_function: functionName,
  });

  shm.writeJson({
    type: "continue",
  });
}

async function mainLoop(): Promise<void> {
  console.error(`[DEBUG] Entering main loop`);

  console.error(`[DEBUG] Sending ready message`);
  shm.writeJson({
    type: "runtime_ready",
    log: "Runtime ready, waiting for commands",
  });

  while (true) {
    const message = shm.waitAndReadJson<Message>(100);

    if (!message) continue;

    console.error(`[DEBUG] Received message: ${JSON.stringify(message)}`);

    switch (message.type) {
      case "load_app":
        console.error(`[DEBUG] Load app command received`);

        shm.writeJson({
          type: "version",
          value: {
            lv: Bun.version_with_sha,
            v: Bun.version,
          },
        });

        if (!appPath) {
          shm.writeJson({
            log: "CHECKPOINT_APP_PATH not set",
            type: "error",
          });
          break;
        }

        shm.writeJson({
          log: `Loading application: ${appPath}`,
        });

        try {
          const absolutePath =
            appPath.startsWith("/") || appPath.startsWith("file://")
              ? appPath
              : `file://${appPath}`;

          console.error(`[DEBUG] Importing: ${absolutePath}`);
          await import(absolutePath);
          console.error(`[DEBUG] App loaded successfully`);

          shm.writeJson({
            log: "Application loaded and ready",
          });
        } catch (error) {
          console.error(`[DEBUG] Import failed: ${error}`);
          shm.writeJson({
            log: `Failed to load app: ${error}`,
            type: "error",
          });
        }
        break;

      case "checkpoint":
        console.error(`[DEBUG] Handling checkpoint`);
        const payload = message.payload as CheckpointPayload;
        if (!payload?.functionName || !Array.isArray(payload.args)) {
          shm.writeJson({
            type: "error",
            message: "Invalid checkpoint payload",
          });
          break;
        }
        handleCheckpoint(payload);
        break;

      case "shutdown":
        console.error(`[DEBUG] Shutdown requested`);
        shm.close();
        process.exit(0);
        break;

      default:
        console.error(`[DEBUG] Unknown message type: '${message.type}'`);
        shm.writeJson({
          type: "error",
          message: "unknown message type",
        });
    }
  }
}

mainLoop();
