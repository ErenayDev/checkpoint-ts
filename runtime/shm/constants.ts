export const HEADER_SIZE = 8;
export const BUFFER_SIZE = 1024 * 1024;
export const SINGLE_CHANNEL_SIZE = HEADER_SIZE + BUFFER_SIZE;
export const TOTAL_SIZE = SINGLE_CHANNEL_SIZE * 2;

export const O_RDWR = 2;
export const PROT_READ = 0x1;
export const PROT_WRITE = 0x2;
export const MAP_SHARED = 0x01;
