# Kernel Upgrade and VFS Templating Plan

This plan outlines the process of moving our Rust filesystem development to the latest Linux kernel (commit `f8f9c1f`, v6.19) and ensuite templating out the full VFS operations.

## User Review Required

> [!WARNING]
> - **Kernel Version Jump**: Moving from v6.8 to v6.19 (commit f8f9c1f) is a significant jump. Rust abstractions may have changed significantly.
> - **Toolchain Re-verification**: We must run `scripts/min-tool-version.sh` and `make rustavailable` again as the required Rust/bindgen versions might have increased.

## Proposed Changes

### [Kernel Upgrade]

- [ ] Fetch commit `f8f9c1f` and create a new branch `rust-fs-v6.19`.
- [ ] Re-apply changes:
    - [ ] `arch/arm64/Kconfig` (add `select HAVE_RUST`)
    - [ ] `fs/Kconfig` (source `fs/hello_rust/Kconfig`)
    - [ ] `fs/Makefile` (add `obj-$(CONFIG_HELLO_RUST_FS) += hello_rust/`)
    - [ ] `fs/hello_rust/` (restore source files)
    - [ ] `scripts/generate_rust_target.rs` (if still needed for arm64)

### [Rust Toolchain]

- [x] Check new version requirements via `scripts/min-tool-version.sh`.
- [/] Update `rustc` to 1.78.0 and re-verify `bindgen-cli` 0.65.1.
  ```bash
  rustup install 1.78.0
  rustup default 1.78.0
  rustup component add rust-src
  ```
- [ ] Verify environment: `make rustavailable`.

### [VFS Templating]

- [ ] Implement a full template in `fs/hello_rust/hello_rust.rs` including:
    - [ ] `FileSystem` registration/unregistration.
    - [ ] `SuperBlock` operations.
    - [ ] `Inode` operations (lookup, create, unlink, mkdir, rmdir, etc.).
    - [ ] `File` operations (read, write, seek, fsync, etc.).
    - [ ] `AddressSpace` operations (if applicable for page cache).

## Verification Plan

### Automated Tests
- `make LLVM=1 rustavailable`
- `make LLVM=1 -j$(nproc) M=fs/hello_rust`

### Manual Verification
1. Boot into the new kernel (`6.19-rc3` or similar).
2. Load module: `sudo modprobe hello_rust`.
3. Verify module load and VFS entry registration in `dmesg`.
4. Attempt to mount to verify super_block initialization.
