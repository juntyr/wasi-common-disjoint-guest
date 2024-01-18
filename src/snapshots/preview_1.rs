#![allow(clippy::manual_async_fn)]

use std::{alloc::Layout, future::Future};

use anyhow::Result;
use wasi_common::snapshots::preview_1::wasi_snapshot_preview1::{self, WasiSnapshotPreview1};

use crate::{ffi, DisjointGuestMemoryAccess, DisjointGuestMemoryAdapter};

macro_rules! impl_syscalls {
    ($($vis:vis async fn $syscall:ident(
        $($param:ident : $ty:ty [as $($transfer:tt)+]),*
    ) -> Result<$ret:ty>;)*) => {
        $(
            $vis fn $syscall<'a>(
                ctx: &'a mut impl WasiSnapshotPreview1,
                memory: &'a DisjointGuestMemoryAdapter<impl DisjointGuestMemoryAccess>,
                $($param: $ty,)*
            ) -> impl Future<Output = Result<$ret>> + 'a {
                async move {
                    let mut alloc = memory.get_alloc()?;

                    let res = impl_syscalls!(impl $syscall(
                        ctx, alloc $(, $param: $ty [as $($transfer)*])*
                    ) <- ()).await?;

                    alloc.finalise()?;

                    Ok(res)
                }
            }
        )*
    };
    (impl $syscall:ident($ctx:ident, $alloc:ident) <- ($($all_param:ident),*)) => {
        wasi_snapshot_preview1::$syscall($ctx, $alloc.get_memory(), $($all_param),*)
    };
    (impl $syscall:ident(
        $ctx:ident, $alloc:ident, $param:ident : $ty:ty [as !]
        $(, $param_rest:ident : $ty_rest:ty [as $($transfer_rest:tt)*])*
    ) <- ($($all_param:ident),*)) => {
        impl_syscalls!(impl $syscall(
            $ctx, $alloc $(, $param_rest: $ty_rest [as $($transfer_rest)*])*
        ) <- ($($all_param),*))
    };
    (impl $syscall:ident(
        $ctx:ident, $alloc:ident, $param:ident : $ty:ty [as const]
        $(, $param_rest:ident : $ty_rest:ty [as $($transfer_rest:tt)*])*
    ) <- ($($all_param:ident),*)) => {
        impl_syscalls!(impl $syscall(
            $ctx, $alloc $(, $param_rest: $ty_rest [as $($transfer_rest)*])*
        ) <- ($($all_param,)* $param))
    };
    (impl $syscall:ident(
        $ctx:ident, $alloc:ident, $param:ident : $ty:ty [as *const [ffi::Iovec; $len:ident]]
        $(, $param_rest:ident : $ty_rest:ty [as $($transfer_rest:tt)*])*
    ) <- ($($all_param:ident),*)) => {
        $alloc.with_ciovs_from_guest($param, $len, move |$alloc, $param| async move {
            impl_syscalls!(impl $syscall(
                $ctx, $alloc $(, $param_rest: $ty_rest [as $($transfer_rest)*])*
            ) <- ($($all_param,)* $param)).await
        })
    };
    (impl $syscall:ident(
        $ctx:ident, $alloc:ident, $param:ident : $ty:ty [as *mut [ffi::Iovec; $len:ident]]
        $(, $param_rest:ident : $ty_rest:ty [as $($transfer_rest:tt)*])*
    ) <- ($($all_param:ident),*)) => {
        $alloc.with_iovs_to_guest($param, $len, move |$alloc, $param| async move {
            impl_syscalls!(impl $syscall(
                $ctx, $alloc $(, $param_rest: $ty_rest [as $($transfer_rest)*])*
            ) <- ($($all_param,)* $param)).await
        })
    };
    (impl $syscall:ident(
        $ctx:ident, $alloc:ident, $param:ident : $ty:ty [as *mut [$transfer:ty; $len:ident]]
        $(, $param_rest:ident : $ty_rest:ty [as $($transfer_rest:tt)*])*
    ) <- ($($all_param:ident),*)) => {
        $alloc.with_write_to_guest($param, Layout::array::<$transfer>(
            usize::try_from(u32::from_ne_bytes($len.to_ne_bytes()))?
        )?, move |$alloc, $param| async move {
            impl_syscalls!(impl $syscall(
                $ctx, $alloc $(, $param_rest: $ty_rest [as $($transfer_rest)*])*
            ) <- ($($all_param,)* $param)).await
        })
    };
    (impl $syscall:ident(
        $ctx:ident, $alloc:ident, $param:ident : $ty:ty [as *mut $transfer:ty]
        $(, $param_rest:ident : $ty_rest:ty [as $($transfer_rest:tt)*])*
    ) <- ($($all_param:ident),*)) => {
        $alloc.with_write_to_guest(
            $param, Layout::new::<$transfer>(), move |$alloc, $param| async move {
                impl_syscalls!(impl $syscall(
                    $ctx, $alloc $(, $param_rest: $ty_rest [as $($transfer_rest)*])*
                ) <- ($($all_param,)* $param)).await
            },
        )
    };
}

impl_syscalls! {
    pub async fn clock_time_get(
        clock_id: i32 [as const],
        precision: i64 [as const],
        time: i32 [as *mut ffi::Timestamp]
    ) -> Result<i32>;

    pub async fn environ_sizes_get(
        environ_size: i32 [as *mut ffi::Size],
        environ_buf_size: i32 [as *mut ffi::Size]
    ) -> Result<i32>;

    pub async fn fd_close(
        fd: i32 [as const]
    ) -> Result<i32>;

    pub async fn fd_fdstat_get(
        fd: i32 [as const],
        fdstat: i32 [as *mut ffi::Fdstat]
    ) -> Result<i32>;

    pub async fn fd_prestat_get(
        fd: i32 [as const],
        prestat: i32 [as *mut ffi::Prestat]
    ) -> Result<i32>;

    pub async fn fd_prestat_dir_name(
        fd: i32 [as const],
        path: i32 [as *mut [u8; path_len]],
        path_len: i32 [as const]
    ) -> Result<i32>;

    pub async fn fd_read(
        fd: i32 [as const],
        iovs: i32 [as *mut [ffi::Iovec; iovs_len]],
        iovs_len: i32 [as const],
        nread: i32 [as *mut ffi::Size]
    ) -> Result<i32>;

    pub async fn fd_seek(
        fd: i32 [as const],
        offset: i64 [as const],
        whence: i32 [as const],
        new_offset: i32 [as *mut ffi::Filesize]
    ) -> Result<i32>;

    pub async fn fd_write(
        fd: i32 [as const],
        ciovs: i32 [as *const [ffi::Iovec; ciovs_len]],
        ciovs_len: i32 [as const],
        nwritten: i32 [as *mut ffi::Size]
    ) -> Result<i32>;

    pub async fn proc_exit(
        rval: i32 [as const]
    ) -> Result<()>;
}

// Note: we need a manual wrapper here to get the sizes first
pub fn environ_get<'a>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemoryAdapter<impl DisjointGuestMemoryAccess>,
    environ: i32,
    environ_buf: i32,
) -> impl Future<Output = Result<i32>> + 'a {
    impl_syscalls! {
        async fn environ_get(
            environ: i32 [as *mut [ffi::Pointer; environ_size]],
            environ_size: u32 [as !],
            environ_buf: i32 [as *mut [u8; environ_buf_size]],
            environ_buf_size: u32 [as !]
        ) -> Result<i32>;
    }

    async move {
        let (environ_size, environ_buf_size) = ctx.environ_sizes_get().await?;

        environ_get(
            ctx,
            memory,
            environ,
            environ_size,
            environ_buf,
            environ_buf_size,
        )
        .await
    }
}
