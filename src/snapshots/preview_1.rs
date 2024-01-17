#![allow(clippy::manual_async_fn)]

use std::{alloc::Layout, future::Future};

use anyhow::Result;
use wasi_common::snapshots::preview_1::wasi_snapshot_preview1::{self, WasiSnapshotPreview1};

use crate::{ffi, DisjointGuestMemory, DisjointGuestMemoryAccess};

pub async fn clock_time_get<'a>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemory<impl DisjointGuestMemoryAccess>,
    clock_id: i32,
    precision: i64,
    time: i32,
) -> impl Future<Output = Result<i32>> + 'a {
    async move {
        let ret = memory
            .with_write_to_guest(time, Layout::new::<ffi::Timestamp>(), |time| {
                wasi_snapshot_preview1::clock_time_get(ctx, memory, clock_id, precision, time)
            })
            .await?;
        memory.clear();
        Ok(ret)
    }
}

pub async fn environ_get<'a>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemory<impl DisjointGuestMemoryAccess>,
    environ: i32,
    environ_buf: i32,
) -> impl Future<Output = Result<i32>> + 'a {
    async move {
        let (environ_size, environ_buf_size) = ctx.environ_sizes_get().await?;
        let ret = memory
            .with_write_to_guest(
                environ,
                Layout::array::<ffi::Pointer>(usize::try_from(environ_size)?)?,
                |environ| async move {
                    #[allow(clippy::let_and_return)]
                    let ret = memory
                        .with_write_to_guest(
                            environ_buf,
                            Layout::array::<u8>(usize::try_from(environ_buf_size)?)?,
                            |environ_buf| {
                                wasi_snapshot_preview1::environ_get(
                                    ctx,
                                    memory,
                                    environ,
                                    environ_buf,
                                )
                            },
                        )
                        .await;
                    ret
                },
            )
            .await?;
        memory.clear();
        Ok(ret)
    }
}

pub async fn environ_sizes_get<'a>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemory<impl DisjointGuestMemoryAccess>,
    environ_size: i32,
    environ_buf_size: i32,
) -> impl Future<Output = Result<i32>> + 'a {
    async move {
        let ret = memory
            .with_write_to_guest(environ_size, Layout::new::<ffi::Size>(), |environ_size| {
                memory.with_write_to_guest(
                    environ_buf_size,
                    Layout::new::<ffi::Size>(),
                    move |environ_buf_size| {
                        wasi_snapshot_preview1::environ_sizes_get(
                            ctx,
                            memory,
                            environ_size,
                            environ_buf_size,
                        )
                    },
                )
            })
            .await?;
        memory.clear();
        Ok(ret)
    }
}

pub async fn fd_close<'a>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemory<impl DisjointGuestMemoryAccess>,
    fd: i32,
) -> impl Future<Output = Result<i32>> + 'a {
    wasi_snapshot_preview1::fd_close(ctx, memory, fd)
}

pub async fn fd_fdstat_get<'a>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemory<impl DisjointGuestMemoryAccess>,
    fd: i32,
    fdstat: i32,
) -> impl Future<Output = Result<i32>> + 'a {
    async move {
        let ret = memory
            .with_write_to_guest(fdstat, Layout::new::<ffi::Fdstat>(), |fdstat| {
                wasi_snapshot_preview1::fd_fdstat_get(ctx, memory, fd, fdstat)
            })
            .await?;
        memory.clear();
        Ok(ret)
    }
}

pub async fn fd_prestat_get<'a>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemory<impl DisjointGuestMemoryAccess>,
    fd: i32,
    prestat: i32,
) -> impl Future<Output = Result<i32>> + 'a {
    async move {
        let ret = memory
            .with_write_to_guest(prestat, Layout::new::<ffi::Prestat>(), |prestat| {
                wasi_snapshot_preview1::fd_prestat_get(ctx, memory, fd, prestat)
            })
            .await?;
        memory.clear();
        Ok(ret)
    }
}

pub async fn fd_prestat_dir_name<'a>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemory<impl DisjointGuestMemoryAccess>,
    fd: i32,
    path: i32,
    path_len: i32,
) -> impl Future<Output = Result<i32>> + 'a {
    async move {
        let path_len_usize = usize::try_from(u32::from_ne_bytes(path_len.to_ne_bytes()))?;
        let ret = memory
            .with_write_to_guest(path, Layout::array::<u8>(path_len_usize)?, |path| {
                wasi_snapshot_preview1::fd_prestat_dir_name(ctx, memory, fd, path, path_len)
            })
            .await?;
        memory.clear();
        Ok(ret)
    }
}

pub async fn fd_read<'a>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemory<impl DisjointGuestMemoryAccess>,
    fd: i32,
    iovs: i32,
    iovs_len: i32,
    nread: i32,
) -> impl Future<Output = Result<i32>> + 'a {
    async move {
        let ret = memory
            .with_iovs_to_guest(iovs, iovs_len, |iovs| {
                memory.with_write_to_guest(nread, Layout::new::<ffi::Size>(), move |nread| {
                    wasi_snapshot_preview1::fd_read(ctx, memory, fd, iovs, iovs_len, nread)
                })
            })
            .await?;
        memory.clear();
        Ok(ret)
    }
}

pub async fn fd_seek<'a>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemory<impl DisjointGuestMemoryAccess>,
    fd: i32,
    offset: i64,
    whence: i32,
    new_offset: i32,
) -> impl Future<Output = Result<i32>> + 'a {
    async move {
        let ret = memory
            .with_write_to_guest(new_offset, Layout::new::<ffi::Filesize>(), |new_offset| {
                wasi_snapshot_preview1::fd_seek(ctx, memory, fd, offset, whence, new_offset)
            })
            .await?;
        memory.clear();
        Ok(ret)
    }
}

pub fn fd_write<'a>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemory<impl DisjointGuestMemoryAccess>,
    fd: i32,
    ciovs: i32,
    ciovs_len: i32,
    nwritten: i32,
) -> impl Future<Output = Result<i32>> + 'a {
    async move {
        let ret = memory
            .with_ciovs_from_guest(ciovs, ciovs_len, |ciovs| {
                memory.with_write_to_guest(nwritten, Layout::new::<ffi::Size>(), move |nwritten| {
                    wasi_snapshot_preview1::fd_write(ctx, memory, fd, ciovs, ciovs_len, nwritten)
                })
            })
            .await?;
        memory.clear();
        Ok(ret)
    }
}

pub fn proc_exit<'a>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemory<impl DisjointGuestMemoryAccess>,
    rval: i32,
) -> impl Future<Output = Result<()>> + 'a {
    wasi_snapshot_preview1::proc_exit(ctx, memory, rval)
}
