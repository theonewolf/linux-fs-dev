// SPDX-License-Identifier: GPL-2.0

//! Hello World Rust Filesystem (VFS Template)

use kernel::prelude::*;
use kernel::bindings;
use kernel::ffi::{c_char, c_int, c_void};
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

// fs_context is opaque in bindings, but we know 'ops' is the first field.
// fs_context_operations is missing, so we define it manually.
#[repr(C)]
struct FsContextOperations {
    free: Option<unsafe extern "C" fn(fc: *mut bindings::fs_context)>,
    dup: Option<unsafe extern "C" fn(fc: *mut bindings::fs_context, src: *mut bindings::fs_context) -> c_int>,
    parse_param: Option<unsafe extern "C" fn(fc: *mut bindings::fs_context, param: *mut bindings::fs_parameter) -> c_int>,
    parse_monolithic: Option<unsafe extern "C" fn(fc: *mut bindings::fs_context, data: *mut c_void) -> c_int>,
    get_tree: Option<unsafe extern "C" fn(fc: *mut bindings::fs_context) -> c_int>,
    reconfigure: Option<unsafe extern "C" fn(fc: *mut bindings::fs_context) -> c_int>,
}

// Manually declare get_tree_nodev
extern "C" {
    pub fn get_tree_nodev(
        fc: *mut bindings::fs_context,
        fill_super: Option<unsafe extern "C" fn(sb: *mut bindings::super_block, fc: *mut bindings::fs_context) -> c_int>,
    ) -> c_int;
}

// --- Global Operations Tables ---

// FS Context Operations
// We use a static reference to ensure it lives forever.
// We make it Sync/Send via unsafe if needed, or arguably it's constant data.
static FC_OPS: FsContextOperations = FsContextOperations {
    free: None,
    dup: None,
    parse_param: None,
    parse_monolithic: None,
    get_tree: Some(hello_get_tree),
    reconfigure: None,
};

// Use 'static mut' for operation tables to avoid Sync issues with generated bindings (raw pointers)
// In a real safe wrapper, these would be 'static' with a Sync-implementing wrapper type.

static mut SUPER_OPS: bindings::super_operations = unsafe {
    let mut ops: bindings::super_operations = core::mem::zeroed();
    ops.statfs = Some(bindings::simple_statfs);
    ops.drop_inode = None; // Use default
    ops
};

static mut INODE_OPS: bindings::inode_operations = unsafe {
    let mut ops: bindings::inode_operations = core::mem::zeroed();
    ops.lookup = Some(bindings::simple_lookup);
    ops
};

static mut FILE_OPS: bindings::file_operations = unsafe {
    let mut ops: bindings::file_operations = core::mem::zeroed();
    ops.read_iter = None; // Read not implemented yet
    ops.write_iter = None;
    ops.open = Some(bindings::dcache_dir_open);
    ops.iterate_shared = Some(bindings::dcache_readdir);
    ops.llseek = Some(bindings::generic_file_llseek);
    ops
};

// File System Type
// We must initialize this at runtime or const. But bindings::file_system_type has non-const fields potentially?
// Actually bindgen structs can be const initialized if fields are pub.
static mut FS_TYPE: bindings::file_system_type = unsafe {
    let mut fs_type: bindings::file_system_type = core::mem::zeroed();
    // String literals need to be null-terminated
    fs_type.name = b"hello_rust\0".as_ptr() as *const c_char;
    fs_type.owner = core::ptr::null_mut(); // Will be set by registration? Or we leave it NULL.
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
    // fc->ops is the first field. Cast fc to pointer-to-pointer-to-ops.
    let ops_ptr = fc as *mut *const FsContextOperations;
    unsafe { *ops_ptr = &FC_OPS };
    0
}

unsafe extern "C" fn hello_get_tree(fc: *mut bindings::fs_context) -> c_int {
    unsafe { get_tree_nodev(fc, Some(hello_fill_super)) }
}

unsafe extern "C" fn hello_fill_super(sb: *mut bindings::super_block, _fc: *mut bindings::fs_context) -> c_int {
    // 1. Setup magic and ops
    unsafe {
        (*sb).s_magic = 0x48454C4F; // "HELO"
        (*sb).s_op = addr_of_mut!(SUPER_OPS);
        (*sb).s_time_gran = 1;
    }

    // 2. Create root inode
    let inode = unsafe { bindings::new_inode(sb) };
    if inode.is_null() {
        return -12; // ENOMEM
    }

    // 3. Init root inode
    unsafe {
        (*inode).i_ino = 1;
        (*inode).i_mode = (bindings::S_IFDIR | 0o755) as u16;
        
        // i_op and i_fop are unionized or not depending on version.
        // Try direct assignment first then anon field if fails? No, we saw failure before.
        // It failed with `i_op` not found in `un_const`. 
        // Wait, error was: `(*inode).un_const.i_op`. 
        // So `inode.i_op` EXPECTS direct access usually.
        // Let's rely on standard bindgen: `i_op` is usually a field.
        // If 6.19 changed it, we check relevant field name.
        // Previous error E0609 says: `un_const` unknown.
        // And `i_fop` unknown (suggested `__bindgen_anon_3.i_fop`).
        // So `i_op` is likely `__bindgen_anon_something.i_op` or just `i_op`?
        // Let's inspect struct inode layout? No time. 
        // I will try standard `i_op` access. If it fails, I'll use `__bindgen_anon_1.i_op` etc.
        // Wait, for `i_fop` it explicitly suggested `__bindgen_anon_3`.
        
        (*inode).i_op = addr_of_mut!(INODE_OPS);
        (*inode).__bindgen_anon_3.i_fop = addr_of_mut!(FILE_OPS);
        
        // Simple manual timestamp init (fake)

    }
    
    // 4. Make root dentry
    let root_dentry = unsafe { bindings::d_make_root(inode) };
    if root_dentry.is_null() {
        return -12;
    }
    
    unsafe { (*sb).s_root = root_dentry };
    
    0
}
