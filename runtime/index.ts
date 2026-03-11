import { SharedMemory } from "./shm/shared-memory";
import { initializeCheckpointRuntime } from "./checkpoint-runtime";

const shmId = process.env.CHECKPOINT_SHM_ID;
if (!shmId) {
  console.error("CHECKPOINT_SHM_ID not set");
  process.exit(1);
}

const shm = SharedMemory.open(shmId);
initializeCheckpointRuntime(shm);

console.error(`[INDEX] Using shared SHM instance: ${shmId}`);

interface Message {
  type: string;
  payload?: unknown;
}

const appPath = process.env.CHECKPOINT_APP_PATH;

async function mainLoop(): Promise<void> {
  console.error(`[INDEX] Entering main loop`);
  console.error(`[INDEX] Sending ready message`);

  shm.sendStatusJson({
    type: "runtime_ready",
    log: "Runtime ready, waiting for commands",
  });

  while (true) {
    const message = await shm.waitReceiveCommandJson<Message>(100);
    if (!message) continue;

    console.error(
      `[INDEX] ← Received command: ${JSON.stringify(message, null, 2)}`,
    );

    switch (message.type) {
      case "load_app": {
        console.error(`[INDEX] Load app command received`);

        shm.sendStatusJson({
          type: "version",
          value: {
            lv: Bun.version_with_sha,
            v: Bun.version,
          },
        });
        console.error(`[INDEX] → Sent: version, ${Bun.version}`);

        if (!appPath) {
          shm.sendStatusJson({
            log: "CHECKPOINT_APP_PATH not set",
            type: "error",
          });
          break;
        }

        shm.sendStatusJson({
          log: `Loading application: ${appPath}`,
        });

        try {
          const absolutePath =
            appPath.startsWith("/") || appPath.startsWith("file://")
              ? appPath
              : `file://${appPath}`;
          console.error(`[INDEX] Importing: ${absolutePath}`);
          await import(absolutePath);
          console.error(`[INDEX] App loaded successfully`);
          shm.sendStatusJson({
            log: "Application loaded and ready",
          });
        } catch (error) {
          console.error(`[INDEX] Import failed: ${error}`);
          shm.sendStatusJson({
            log: `Failed to load app: ${error}`,
            type: "error",
          });
        }
        break;
      }
      case "shutdown":
        console.error(`[INDEX] Shutdown requested`);
        shm.close();
        process.exit(0);
        break;

      default:
        console.error(`[INDEX] Unknown message type: '${message.type}'`);
        shm.sendStatusJson({
          type: "error",
          log: `Unknown message type: ${message.type}`,
        });
    }
  }
}

mainLoop();
