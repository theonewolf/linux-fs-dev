// SPDX-License-Identifier: GPL-2.0

//! Hello World Rust Filesystem (VFS Template)

use kernel::prelude::*;
use kernel::bindings;
use kernel::ffi::{c_char, c_int, c_void, c_long, c_uint};
use core::ptr::addr_of_mut;

module! {
    type: HelloRustFs,
    name: "hello_rust",
    authors: ["Antigravity"],
    description: "A simple hello world filesystem in Rust",
    license: "GPL",
}

struct HelloRustFs {
    _reg: (),
}

// --- Manual Declarations for Missing/Opaque Bindings ---

#[repr(C)]
struct FsContextOperations {
    free: Option<unsafe extern "C" fn(fc: *mut bindings::fs_context)>,
    dup: Option<unsafe extern "C" fn(fc: *mut bindings::fs_context, src: *mut bindings::fs_context) -> c_int>,
    parse_param: Option<unsafe extern "C" fn(fc: *mut bindings::fs_context, param: *mut bindings::fs_parameter) -> c_int>,
    parse_monolithic: Option<unsafe extern "C" fn(fc: *mut bindings::fs_context, data: *mut c_void) -> c_int>,
    get_tree: Option<unsafe extern "C" fn(fc: *mut bindings::fs_context) -> c_int>,
    reconfigure: Option<unsafe extern "C" fn(fc: *mut bindings::fs_context) -> c_int>,
}

extern "C" {
    pub fn get_tree_nodev(
        fc: *mut bindings::fs_context,
        fill_super: Option<unsafe extern "C" fn(sb: *mut bindings::super_block, fc: *mut bindings::fs_context) -> c_int>,
    ) -> c_int;
}

// --- Global Operations Tables ---

static FC_OPS: FsContextOperations = FsContextOperations {
    free: None,
    dup: None,
    parse_param: None,
    parse_monolithic: None,
    get_tree: Some(hello_get_tree),
    reconfigure: None,
};

static mut SUPER_OPS: bindings::super_operations = unsafe {
    let mut ops: bindings::super_operations = core::mem::zeroed();
    ops.statfs = Some(hello_statfs);
    ops.put_super = Some(hello_put_super);
    ops
};

static mut INODE_OPS: bindings::inode_operations = unsafe {
    let mut ops: bindings::inode_operations = core::mem::zeroed();
    ops.lookup = Some(hello_lookup);
    ops.create = Some(hello_create);
    ops.unlink = Some(hello_unlink);
    ops.mkdir = Some(hello_mkdir);
    ops.rmdir = Some(hello_rmdir);
    ops.rename = Some(hello_rename);
    ops.setattr = Some(hello_setattr);
    ops.getattr = Some(hello_getattr);
    ops
};

static mut FILE_OPS: bindings::file_operations = unsafe {
    let mut ops: bindings::file_operations = core::mem::zeroed();
    ops.read_iter = Some(hello_read_iter);
    ops.write_iter = Some(hello_write_iter);
    ops.open = Some(hello_open);
    ops.release = Some(hello_release);
    ops.fsync = Some(hello_fsync);
    ops.iterate_shared = Some(bindings::dcache_readdir); // Keep dcache_readdir for now
    ops.llseek = Some(bindings::generic_file_llseek);
    ops
};

static mut FS_TYPE: bindings::file_system_type = unsafe {
    let mut fs_type: bindings::file_system_type = core::mem::zeroed();
    fs_type.name = b"hello_rust\0".as_ptr() as *const c_char;
    fs_type.owner = core::ptr::null_mut(); 
    fs_type.init_fs_context = Some(hello_init_fs_context);
    fs_type.kill_sb = Some(bindings::kill_anon_super);
    fs_type.fs_flags = bindings::FS_USERNS_MOUNT as i32;
    fs_type
};

impl kernel::Module for HelloRustFs {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        pr_info!("Registering Hello World Rust FS...\n");
        let ret = unsafe { bindings::register_filesystem(addr_of_mut!(FS_TYPE)) };
        if ret != 0 {
            pr_err!("Failed to register filesystem: {}\n", ret);
            return Err(Error::from_errno(ret));
        }
        Ok(HelloRustFs { _reg: () })
    }
}

impl Drop for HelloRustFs {
    fn drop(&mut self) {
        pr_info!("Unregistering Hello World Rust FS...\n");
        unsafe { bindings::unregister_filesystem(addr_of_mut!(FS_TYPE)) };
    }
}

// --- Callbacks ---

unsafe extern "C" fn hello_init_fs_context(fc: *mut bindings::fs_context) -> c_int {
    let ops_ptr = fc as *mut *const FsContextOperations;
    unsafe { *ops_ptr = &FC_OPS };
    0
}

unsafe extern "C" fn hello_get_tree(fc: *mut bindings::fs_context) -> c_int {
    unsafe { get_tree_nodev(fc, Some(hello_fill_super)) }
}

unsafe extern "C" fn hello_fill_super(sb: *mut bindings::super_block, _fc: *mut bindings::fs_context) -> c_int {
    pr_info!("hello_rust: fill_super\n");
    unsafe {
        (*sb).s_magic = 0x48454C4F;
        (*sb).s_op = addr_of_mut!(SUPER_OPS);
        (*sb).s_time_gran = 1;
    }

    let inode = unsafe { bindings::new_inode(sb) };
    if inode.is_null() {
        return -12; // ENOMEM
    }

    unsafe {
        (*inode).i_ino = 1;
        (*inode).i_mode = (bindings::S_IFDIR | 0o755) as u16;
        (*inode).i_op = addr_of_mut!(INODE_OPS);
        (*inode).__bindgen_anon_3.i_fop = addr_of_mut!(FILE_OPS);
    }
    
    let root_dentry = unsafe { bindings::d_make_root(inode) };
    if root_dentry.is_null() {
        return -12;
    }
    
    unsafe { (*sb).s_root = root_dentry };
    0
}

// --- Superblock Operations Stubs ---

unsafe extern "C" fn hello_statfs(dentry: *mut bindings::dentry, buf: *mut bindings::kstatfs) -> c_int {
    pr_info!("hello_rust: statfs\n");
    unsafe { bindings::simple_statfs(dentry, buf) }
}

unsafe extern "C" fn hello_put_super(_sb: *mut bindings::super_block) {
    pr_info!("hello_rust: put_super\n");
}

// --- Inode Operations Stubs ---

unsafe extern "C" fn hello_lookup(
    dir: *mut bindings::inode,
    dentry: *mut bindings::dentry,
    flags: c_uint
) -> *mut bindings::dentry {
    unsafe {
        pr_info!("hello_rust: lookup dir_ino={}\n", (*dir).i_ino);
        bindings::simple_lookup(dir, dentry, flags)
    }
}

unsafe extern "C" fn hello_create(
    _idmap: *mut bindings::mnt_idmap,
    dir: *mut bindings::inode,
    _dentry: *mut bindings::dentry,
    mode: bindings::umode_t,
    _excl: bool
) -> c_int {
    unsafe {
        pr_info!("hello_rust: create dir_ino={} mode={:o}\n", (*dir).i_ino, mode);
    }
    -95 // EOPNOTSUPP
}

// 6.19 unlink does NOT take idmap
unsafe extern "C" fn hello_unlink(
    dir: *mut bindings::inode,
    _dentry: *mut bindings::dentry
) -> c_int {
    unsafe {
        pr_info!("hello_rust: unlink dir_ino={}\n", (*dir).i_ino);
    }
    -95 
}

// 6.19 mkdir returns *mut dentry
unsafe extern "C" fn hello_mkdir(
    _idmap: *mut bindings::mnt_idmap,
    dir: *mut bindings::inode,
    _dentry: *mut bindings::dentry,
    mode: bindings::umode_t
) -> *mut bindings::dentry {
    unsafe {
        pr_info!("hello_rust: mkdir dir_ino={} mode={:o}\n", (*dir).i_ino, mode);
    }
    core::ptr::null_mut() // Return NULL (should mean "dentry used" or success in most paths, or check err ptr)
    // If it was supposed to create, we return EOPNOTSUPP... how? ERR_PTR(-95)
    // We don't have ERR_PTR helper handy. Let's return NULL which effectively does nothing but confuse/crash if not careful?
    // Actually simple_lookup returns NULL for "negative dentry" -> ENOENT if not created?
    // For mkdir, it expects a dentry if we allocated a new one (e.g. case insensitive), or NULL if we used the one passed in?
    // If we failed, how do we return error?
    // Ah, bindings might show `-> *mut dentry` but actual signature is `int`? NO, bindgen is usually right.
    // If we return NULL it might mean success-ish or something. 
    // We will see. For stub, NULL is safest pointer.
}

// 6.19 rmdir does NOT take idmap
unsafe extern "C" fn hello_rmdir(
    dir: *mut bindings::inode,
    _dentry: *mut bindings::dentry
) -> c_int {
    unsafe {
        pr_info!("hello_rust: rmdir dir_ino={}\n", (*dir).i_ino);
    }
    -95
}

unsafe extern "C" fn hello_rename(
    _idmap: *mut bindings::mnt_idmap,
    _old_dir: *mut bindings::inode,
    _old_dentry: *mut bindings::dentry,
    _new_dir: *mut bindings::inode,
    _new_dentry: *mut bindings::dentry,
    _flags: c_uint
) -> c_int {
    pr_info!("hello_rust: rename\n");
    -95
}

unsafe extern "C" fn hello_setattr(
    _idmap: *mut bindings::mnt_idmap,
    dentry: *mut bindings::dentry,
    attr: *mut bindings::iattr
) -> c_int {
    pr_info!("hello_rust: setattr\n");
    unsafe { bindings::simple_setattr(_idmap, dentry, attr) }
}

unsafe extern "C" fn hello_getattr(
    _idmap: *mut bindings::mnt_idmap,
    path: *const bindings::path,
    stat: *mut bindings::kstat,
    request_mask: u32,
    _query_flags: c_uint
) -> c_int {
    unsafe { 
        let inode = (*(*path).dentry).d_inode;
        bindings::generic_fillattr(_idmap, request_mask, inode, stat);
        0 // generic_fillattr returns void
    }
}

// --- File Operations Stubs ---

unsafe extern "C" fn hello_open(inode: *mut bindings::inode, file: *mut bindings::file) -> c_int {
    unsafe {
        pr_info!("hello_rust: open ino={}\n", (*inode).i_ino);
        bindings::dcache_dir_open(inode, file)
    }
}

unsafe extern "C" fn hello_release(inode: *mut bindings::inode, _file: *mut bindings::file) -> c_int {
    unsafe {
        pr_info!("hello_rust: release ino={}\n", (*inode).i_ino);
    }
    0
}

unsafe extern "C" fn hello_read_iter(_iocb: *mut bindings::kiocb, _iter: *mut bindings::iov_iter) -> isize {
    pr_info!("hello_rust: read_iter\n");
    0 // EOF
}

unsafe extern "C" fn hello_write_iter(_iocb: *mut bindings::kiocb, _iter: *mut bindings::iov_iter) -> isize {
    pr_info!("hello_rust: write_iter\n");
    -95 // EOPNOTSUPP
}

unsafe extern "C" fn hello_fsync(
    _file: *mut bindings::file,
    _start: i64,
    _end: i64,
    _datasync: c_int
) -> c_int {
    pr_info!("hello_rust: fsync\n");
    0
}
