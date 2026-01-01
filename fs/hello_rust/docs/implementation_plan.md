# VFS Operation Stubs Implementation Plan

## Goal Description
Implement stub methods for essential VFS operations in `hello_rust.rs`. Each stub will:
1.  Log its invocation to `dmesg` (e.g., "hello_rust: open called").
2.  Return a success code (`0`) or specific error (e.g., `-EPERM`) as appropriate.
3.  Be documented with inputs and expected return values.

## Proposed Changes

### [fs/hello_rust/hello_rust.rs]
- **Superblock Operations**:
    - `statfs`: Already stubbed, add logging.
    - `drop_inode`: Add logging.
    - `alloc_inode` / `destroy_inode`: Implement simple slab allocation stubs (or use generic if possible, but stubs requested).
    - `put_super`: usage logging.
- **Inode Operations**:
    - `create`: Log and return `-ENOSYS` (not implemented yet, but stubbed).
    - `lookup`: Log and return NULL (or simple ENOENT).
    - `unlink`, `mkdir`, `rmdir`: Log and return success or error.
    - `rename`, `setattr`, `getattr`: Stubs.
- **File Operations**:
    - `read_iter`: Log and return 0 (EOF).
    - `write_iter`: Log and return count (fake write) or error.
    - `open`: Log.
    - `release`: Log.
    - `fsync`: Log.

## Verification Plan

### Automated Tests
- Build module: `make LLVM=1 M=fs/hello_rust`

### Manual Verification
1.  Load module: `insmod hello_rust.ko`.
2.  Mount: `mount -t hello_rust none /tmp/hello`.
3.  Trigger Operations:
    - `ls -la /tmp/hello` (triggers `lookup`, `getattr`, `opendir`, `readdir`).
    - `touch /tmp/hello/test` (triggers `create`).
    - `mkdir /tmp/hello/dir` (triggers `mkdir`).
    - `echo "test" > /tmp/hello/file` (triggers `open`, `write`).
4.  Check Logs: `dmesg | tail` to see the "hello_rust: ..." messages.
5.  Unmount and unload.
