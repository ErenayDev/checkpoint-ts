export const QUEUE_CAPACITY = 16;
export const MESSAGE_SIZE = 64 * 1024;
export const RING_BUFFER_HEADER_SIZE = 32;
export const SLOT_HEADER_SIZE = 8;
export const SLOT_SIZE = MESSAGE_SIZE;
export const SLOTS_TOTAL_SIZE = SLOT_SIZE * QUEUE_CAPACITY;
export const SINGLE_CHANNEL_SIZE = RING_BUFFER_HEADER_SIZE + SLOTS_TOTAL_SIZE;
export const NUM_CHANNELS = 4;
export const TOTAL_SIZE = SINGLE_CHANNEL_SIZE * NUM_CHANNELS;

export const O_RDWR = 2;
export const PROT_READ = 0x1;
export const PROT_WRITE = 0x2;
export const MAP_SHARED = 0x01;

export enum Channel {
  RustToTsCommand = 0,
  TsToRustStatus = 1,
  TsToRustCheckpoint = 2,
  RustToTsCheckpointResponse = 3,
}
