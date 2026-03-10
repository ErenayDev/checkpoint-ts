import { ptr, toArrayBuffer } from "bun:ffi";
import { libc } from "./ffi";
import {
  RING_BUFFER_HEADER_SIZE,
  MESSAGE_SIZE,
  SLOT_HEADER_SIZE,
  SLOT_SIZE,
  SINGLE_CHANNEL_SIZE,
  TOTAL_SIZE,
  QUEUE_CAPACITY,
  O_RDWR,
  PROT_READ,
  PROT_WRITE,
  MAP_SHARED,
  Channel,
} from "./constants";

export class SharedMemory {
  private pointer: number;
  private buffer: Uint8Array;
  private name: string;

  private constructor(name: string, pointer: number) {
    this.name = name;
    this.pointer = pointer;
    this.buffer = new Uint8Array(toArrayBuffer(pointer as any, 0, TOTAL_SIZE));
  }

  static open(shmName: string): SharedMemory {
    const nameBuffer = Buffer.from(shmName + "\0");
    const fd = libc.symbols.shm_open(ptr(nameBuffer), O_RDWR, 0o666);

    if (fd < 0) {
      throw new Error(`shm_open failed: ${shmName}`);
    }

    const address = libc.symbols.mmap(
      null,
      TOTAL_SIZE,
      PROT_READ | PROT_WRITE,
      MAP_SHARED,
      fd,
      0,
    );

    libc.symbols.close(fd);

    if (address === 0 || address === -1) {
      throw new Error("mmap failed");
    }

    return new SharedMemory(shmName, Number(address));
  }

  private getChannelOffset(channel: Channel): number {
    return channel * SINGLE_CHANNEL_SIZE;
  }

  private getSlotOffset(channel: Channel, slotIndex: number): number {
    return (
      this.getChannelOffset(channel) +
      RING_BUFFER_HEADER_SIZE +
      slotIndex * SLOT_SIZE
    );
  }

  private writeToSlot(
    channel: Channel,
    slotIndex: number,
    data: Uint8Array,
  ): void {
    if (data.length > MESSAGE_SIZE - SLOT_HEADER_SIZE) {
      throw new Error("data exceeds maximum message size");
    }

    const slotOffset = this.getSlotOffset(channel, slotIndex);
    const readyFlagArray = new Uint32Array(this.buffer.buffer, slotOffset, 1);
    const view = new DataView(this.buffer.buffer, slotOffset);

    Atomics.store(readyFlagArray, 0, 0);

    view.setUint32(4, data.length, true);

    this.buffer.set(data, slotOffset + SLOT_HEADER_SIZE);

    Atomics.store(readyFlagArray, 0, 1);
  }

  private readFromSlot(channel: Channel, slotIndex: number): Uint8Array {
    const slotOffset = this.getSlotOffset(channel, slotIndex);
    const readyFlagArray = new Uint32Array(this.buffer.buffer, slotOffset, 1);

    let attempts = 0;
    while (Atomics.load(readyFlagArray, 0) !== 1) {
      if (attempts++ > 100000) {
        console.error(`[SLOT-READ] Timeout waiting for slot ${slotIndex}`);
        return new Uint8Array(0);
      }
    }

    const view = new DataView(this.buffer.buffer, slotOffset);
    const dataLength = view.getUint32(4, true);

    if (dataLength > MESSAGE_SIZE - SLOT_HEADER_SIZE) {
      return new Uint8Array(0);
    }

    return this.buffer.slice(
      slotOffset + SLOT_HEADER_SIZE,
      slotOffset + SLOT_HEADER_SIZE + dataLength,
    );
  }

  private enqueueSPSC(channel: Channel, data: Uint8Array): void {
    this.ensureOpen();

    const channelOffset = this.getChannelOffset(channel);
    const writeIndexArray = new Uint32Array(
      this.buffer.buffer,
      channelOffset,
      1,
    );
    const readIndexArray = new Uint32Array(
      this.buffer.buffer,
      channelOffset + 4,
      1,
    );

    const currentWriteIndex = Atomics.load(writeIndexArray, 0);
    const currentReadIndex = Atomics.load(readIndexArray, 0);

    const nextWriteIndex = (currentWriteIndex + 1) % QUEUE_CAPACITY;

    if (nextWriteIndex === currentReadIndex) {
      console.error(
        `[SPSC-TS] Queue full! write=${currentWriteIndex}, read=${currentReadIndex}`,
      );
      throw new Error("queue full");
    }

    this.writeToSlot(channel, currentWriteIndex, data);

    Atomics.store(writeIndexArray, 0, nextWriteIndex);
  }
  private static readonly MAX_CAS_RETRIES = 10000;

  private enqueueMPSC(channel: Channel, data: Uint8Array): void {
    this.ensureOpen();

    if (data.length > MESSAGE_SIZE - SLOT_HEADER_SIZE) {
      throw new Error("data exceeds maximum message size");
    }

    const channelOffset = this.getChannelOffset(channel);
    const writeIndexArray = new Uint32Array(
      this.buffer.buffer,
      channelOffset,
      1,
    );
    const readIndexArray = new Uint32Array(
      this.buffer.buffer,
      channelOffset + 4,
      1,
    );

    let attempts = 0;
    while (attempts < SharedMemory.MAX_CAS_RETRIES) {
      const currentWriteIndex = Atomics.load(writeIndexArray, 0);
      const currentReadIndex = Atomics.load(readIndexArray, 0);

      const nextWriteIndex = (currentWriteIndex + 1) % QUEUE_CAPACITY;

      if (nextWriteIndex === currentReadIndex) {
        throw new Error("queue full");
      }

      const previousValue = Atomics.compareExchange(
        writeIndexArray,
        0,
        currentWriteIndex,
        nextWriteIndex,
      );

      if (previousValue === currentWriteIndex) {
        this.writeToSlot(channel, currentWriteIndex, data);
        return;
      }

      attempts++;
    }

    throw new Error(`MPSC enqueue failed after ${attempts} CAS retries`);
  }

  private dequeue(channel: Channel): Uint8Array | null {
    this.ensureOpen();

    const channelOffset = this.getChannelOffset(channel);
    const writeIndexArray = new Uint32Array(
      this.buffer.buffer,
      channelOffset,
      1,
    );
    const readIndexArray = new Uint32Array(
      this.buffer.buffer,
      channelOffset + 4,
      1,
    );

    const currentReadIndex = Atomics.load(readIndexArray, 0);
    const currentWriteIndex = Atomics.load(writeIndexArray, 0);

    if (currentReadIndex === currentWriteIndex) {
      return null;
    }

    console.error(
      `[DEQUEUE-TS] Reading slot[${currentReadIndex}], read_idx=${currentReadIndex}, write_idx=${currentWriteIndex}`,
    );

    const data = this.readFromSlot(channel, currentReadIndex);

    const nextReadIndex = (currentReadIndex + 1) % QUEUE_CAPACITY;
    Atomics.store(readIndexArray, 0, nextReadIndex);

    console.error(
      `[DEQUEUE-TS] Slot[${currentReadIndex}] read complete, read_idx: ${currentReadIndex}→${nextReadIndex}`,
    );

    return data;
  }

  private async waitDequeue(
    channel: Channel,
    timeoutMs: number,
  ): Promise<Uint8Array | null> {
    const start = performance.now();

    while (performance.now() - start < timeoutMs) {
      const data = this.dequeue(channel);
      if (data) return data;
      await Bun.sleep(1);
    }

    return null;
  }

  receiveCommand(): Uint8Array | null {
    return this.dequeue(Channel.RustToTsCommand);
  }

  receiveCommandJson<T>(): T | null {
    const data = this.receiveCommand();
    if (!data) return null;
    try {
      return JSON.parse(new TextDecoder().decode(data));
    } catch (error) {
      console.error("Failed to parse command JSON:", error);
      return null;
    }
  }

  async waitReceiveCommand(timeoutMs: number): Promise<Uint8Array | null> {
    return this.waitDequeue(Channel.RustToTsCommand, timeoutMs);
  }

  async waitReceiveCommandJson<T>(timeoutMs: number): Promise<T | null> {
    const data = await this.waitReceiveCommand(timeoutMs);
    if (!data) return null;
    try {
      return JSON.parse(new TextDecoder().decode(data));
    } catch (error) {
      console.error("Failed to parse command JSON:", error);
      return null;
    }
  }

  sendStatus(data: Uint8Array): void {
    try {
      this.enqueueSPSC(Channel.TsToRustStatus, data);
    } catch (error) {
      console.error("Failed to send status:", error);
      throw error;
    }
  }

  sendStatusJson(obj: unknown): void {
    let jsonStr: string;
    try {
      jsonStr = JSON.stringify(obj, (key, value) => {
        if (typeof value === "bigint") return value.toString();
        return value;
      });
    } catch (error) {
      jsonStr = JSON.stringify({
        error: "Serialization failed",
        message: String(error),
      });
    }
    const data = new TextEncoder().encode(jsonStr);
    this.sendStatus(data);
  }

  sendCheckpoint(data: Uint8Array): void {
    try {
      this.enqueueMPSC(Channel.TsToRustCheckpoint, data);
    } catch (error) {
      console.error("Failed to send checkpoint:", error);
      throw error;
    }
  }

  sendCheckpointJson(obj: unknown): void {
    let jsonStr: string;
    try {
      jsonStr = JSON.stringify(obj, (key, value) => {
        if (typeof value === "bigint") return value.toString();
        return value;
      });
    } catch (error) {
      jsonStr = JSON.stringify({
        error: "Serialization failed",
        message: String(error),
      });
    }
    const data = new TextEncoder().encode(jsonStr);
    this.sendCheckpoint(data);
  }

  receiveCheckpointResponse(): Uint8Array | null {
    return this.dequeue(Channel.RustToTsCheckpointResponse);
  }

  receiveCheckpointResponseJson<T>(): T | null {
    const data = this.receiveCheckpointResponse();
    if (!data) return null;
    try {
      return JSON.parse(new TextDecoder().decode(data));
    } catch (error) {
      console.error("Failed to parse checkpoint response JSON:", error);
      return null;
    }
  }

  async waitReceiveCheckpointResponse(
    timeoutMs: number,
  ): Promise<Uint8Array | null> {
    return this.waitDequeue(Channel.RustToTsCheckpointResponse, timeoutMs);
  }

  async waitReceiveCheckpointResponseJson<T>(
    timeoutMs: number,
  ): Promise<T | null> {
    const data = await this.waitReceiveCheckpointResponse(timeoutMs);
    if (!data) return null;
    try {
      return JSON.parse(new TextDecoder().decode(data));
    } catch (error) {
      console.error("Failed to parse checkpoint response JSON:", error);
      return null;
    }
  }

  debugQueueState(): void {
    console.error("[DEBUG] ===== TypeScript Queue State =====");

    for (let channel = 0; channel < 4; channel++) {
      const channelOffset = this.getChannelOffset(channel);
      const writeIndexArray = new Uint32Array(
        this.buffer.buffer,
        channelOffset,
        1,
      );
      const readIndexArray = new Uint32Array(
        this.buffer.buffer,
        channelOffset + 4,
        1,
      );

      const writeIdx = Atomics.load(writeIndexArray, 0);
      const readIdx = Atomics.load(readIndexArray, 0);

      const count =
        writeIdx >= readIdx
          ? writeIdx - readIdx
          : QUEUE_CAPACITY - readIdx + writeIdx;

      const channelNames = [
        "RustToTsCommand",
        "TsToRustStatus",
        "TsToRustCheckpoint",
        "RustToTsCheckpointResponse",
      ];

      console.error(
        `[DEBUG] Channel ${channel} (${channelNames[channel]}): write=${writeIdx}, read=${readIdx}, queued=${count}/${QUEUE_CAPACITY}`,
      );
    }

    console.error("[DEBUG] ====================================");
  }

  close(): void {
    if (this.pointer === 0) {
      return;
    }

    const rc = libc.symbols.munmap(this.pointer, TOTAL_SIZE);
    if (rc !== 0) {
      throw new Error("munmap failed");
    }

    this.pointer = 0;
    this.buffer = new Uint8Array(0);
  }

  private ensureOpen(): void {
    if (this.pointer === 0) {
      throw new Error("SharedMemory has been closed");
    }
  }
}
