import { ptr, toArrayBuffer } from "bun:ffi";
import { libc } from "./ffi";
import {
  HEADER_SIZE,
  BUFFER_SIZE,
  SINGLE_CHANNEL_SIZE,
  TOTAL_SIZE,
  O_RDWR,
  PROT_READ,
  PROT_WRITE,
  MAP_SHARED,
} from "./constants";

export class SharedMemory {
  private pointer: number;
  private buffer: Uint8Array;
  private name: string; // use later

  private constructor(name: string, pointer: number) {
    this.name = name;
    this.pointer = pointer;
    this.buffer = new Uint8Array(toArrayBuffer(pointer, 0, TOTAL_SIZE));
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
      // address === 0 || address === 0xffffffffffffffffn)
      throw new Error("mmap failed");
    }

    return new SharedMemory(shmName, Number(address));
  }

  read(): Uint8Array | null {
    this.ensureOpen(); // check if sharedmemory is open
    const view = new DataView(this.buffer.buffer, 0);
    const flagArray = new Uint32Array(this.buffer.buffer, 0, 1);
    const readyFlag = Atomics.load(flagArray, 0);

    if (readyFlag === 0) {
      return null;
    }

    const dataLen = view.getUint32(4, true);
    const data = this.buffer.slice(HEADER_SIZE, HEADER_SIZE + dataLen);

    Atomics.store(flagArray, 0, 0);

    return data;
  }

  write(data: Uint8Array): void {
    this.ensureOpen(); // check if sharedmemory is open
    if (data.length > BUFFER_SIZE) {
      throw new Error("data exceeds buffer size");
    }

    const offset = SINGLE_CHANNEL_SIZE;
    const view = new DataView(this.buffer.buffer, offset);
    const flagArray = new Uint32Array(this.buffer.buffer, offset, 1);

    this.buffer.set(data, offset + HEADER_SIZE);
    view.setUint32(4, data.length, true);
    Atomics.store(flagArray, 0, 1);
  }

  readJson<T>(): T | null {
    const data = this.read();
    if (!data) return null;
    return JSON.parse(new TextDecoder().decode(data));
  }

  writeJson(obj: unknown): void {
    const data = new TextEncoder().encode(JSON.stringify(obj));
    this.write(data);
  }

  waitAndRead(timeoutMs: number): Uint8Array | null {
    const start = performance.now();

    while (performance.now() - start < timeoutMs) {
      const data = this.read();
      if (data) return data;
    }

    return null;
  }

  waitAndReadJson<T>(timeoutMs: number): T | null {
    const data = this.waitAndRead(timeoutMs);
    if (!data) return null;
    return JSON.parse(new TextDecoder().decode(data));
  }

  close(): void {
    libc.symbols.munmap(this.pointer, TOTAL_SIZE);
    this.pointer = 0;
    this.buffer = new Uint8Array(0);
  }

  private ensureOpen(): void {
    if (this.pointer === 0) {
      throw new Error("SharedMemory has been closed");
    }
  }
}
