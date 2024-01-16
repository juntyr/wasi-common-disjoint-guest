pub type Pointer = Size;
pub type Size = wasi_common::snapshots::preview_1::types::Size;
pub type Timestamp = wasi::Timestamp;
pub type Filesize = wasi::Filesize;
pub type Fdstat = wasi::Fdstat;
pub type Prestat = stable::Prestat;
pub type Iovec = stable::Iovec;

#[allow(dead_code)]
mod stable {
    use super::*;

    #[repr(C)]
    #[derive(Copy, Clone)]
    struct PrestatDir {
        pr_name_len: Size,
    }
    #[repr(C)]
    union PrestatU {
        dir: PrestatDir,
    }
    #[repr(C)]
    pub struct Prestat {
        tag: u8,
        u: PrestatU,
    }

    #[repr(C)]
    pub struct Iovec {
        buf: Pointer,
        buf_len: Size,
    }
}
