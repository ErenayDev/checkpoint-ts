import { dlopen, FFIType } from "bun:ffi";

const libcPath =
  process.platform === "linux" ? "libc.so.6" : "libSystem.B.dylib";

export const libc = dlopen(libcPath, {
  shm_open: {
    args: [FFIType.ptr, FFIType.i32, FFIType.i32],
    returns: FFIType.i32,
  },
  ftruncate: {
    args: [FFIType.i32, FFIType.i64],
    returns: FFIType.i32,
  },
  mmap: {
    args: [
      FFIType.ptr,
      FFIType.u64,
      FFIType.i32,
      FFIType.i32,
      FFIType.i32,
      FFIType.i64,
    ],
    returns: FFIType.ptr,
  },
  munmap: {
    args: [FFIType.ptr, FFIType.u64],
    returns: FFIType.i32,
  },
  close: {
    args: [FFIType.i32],
    returns: FFIType.i32,
  },
  shm_unlink: {
    args: [FFIType.ptr],
    returns: FFIType.i32,
  },
});
