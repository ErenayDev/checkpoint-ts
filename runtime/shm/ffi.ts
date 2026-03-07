import { dlopen, FFIType } from "bun:ffi";

// support linux & macos
const libcPath = (() => {
    switch (process.platform) {
        case "linux":
            return "libc.so.6";
        case "darwin":
            return "libSystem.B.dylib";
        default:
            throw new Error(`
Unsupported platform for shared memory: ${process.platform}`);
    }
})();

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
