#![allow(clippy::manual_async_fn)]

use std::{alloc::Layout, future::Future};

use anyhow::Result;
use wasi_common::snapshots::preview_1::wasi_snapshot_preview1::{self, WasiSnapshotPreview1};

use crate::{ffi, DisjointGuestMemoryAccess, DisjointGuestMemoryAdapter};

pub fn clock_time_get<'a>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemoryAdapter<impl DisjointGuestMemoryAccess + 'static>,
    clock_id: i32,
    precision: i64,
    time: i32,
) -> impl Future<Output = Result<i32>> + 'a {
    async move {
        let mut alloc = memory.get_alloc()?;

        let ret = alloc
            .with_write_to_guest(time, Layout::new::<ffi::Timestamp>(), |alloc, time| {
                wasi_snapshot_preview1::clock_time_get(
                    ctx,
                    alloc.as_memory(),
                    clock_id,
                    precision,
                    time,
                )
            })
            .await?;

        alloc.finalise()?;

        Ok(ret)
    }
}

pub fn environ_get<'a>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemoryAdapter<impl DisjointGuestMemoryAccess>,
    environ: i32,
    environ_buf: i32,
) -> impl Future<Output = Result<i32>> + 'a {
    async move {
        let (environ_size, environ_buf_size) = ctx.environ_sizes_get().await?;

        let mut alloc = memory.get_alloc()?;

        let ret = alloc
            .with_write_to_guest(
                environ,
                Layout::array::<ffi::Pointer>(usize::try_from(environ_size)?)?,
                |alloc, environ| async move {
                    #[allow(clippy::let_and_return)]
                    let ret = alloc
                        .with_write_to_guest(
                            environ_buf,
                            Layout::array::<u8>(usize::try_from(environ_buf_size)?)?,
                            |alloc, environ_buf| {
                                wasi_snapshot_preview1::environ_get(
                                    ctx,
                                    alloc.as_memory(),
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

        alloc.finalise()?;

        Ok(ret)
    }
}

pub fn environ_sizes_get<'a>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemoryAdapter<impl DisjointGuestMemoryAccess>,
    environ_size: i32,
    environ_buf_size: i32,
) -> impl Future<Output = Result<i32>> + 'a {
    async move {
        let mut alloc = memory.get_alloc()?;

        let ret = alloc
            .with_write_to_guest(
                environ_size,
                Layout::new::<ffi::Size>(),
                |alloc, environ_size| {
                    alloc.with_write_to_guest(
                        environ_buf_size,
                        Layout::new::<ffi::Size>(),
                        move |alloc, environ_buf_size| {
                            wasi_snapshot_preview1::environ_sizes_get(
                                ctx,
                                alloc.as_memory(),
                                environ_size,
                                environ_buf_size,
                            )
                        },
                    )
                },
            )
            .await?;

        alloc.finalise()?;

        Ok(ret)
    }
}

pub fn fd_close<'a>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemoryAdapter<impl DisjointGuestMemoryAccess>,
    fd: i32,
) -> impl Future<Output = Result<i32>> + 'a {
    async move {
        let mut alloc = memory.get_alloc()?;

        let res = wasi_snapshot_preview1::fd_close(ctx, alloc.as_memory(), fd).await?;

        alloc.finalise()?;

        Ok(res)
    }
}

pub fn fd_fdstat_get<'a>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemoryAdapter<impl DisjointGuestMemoryAccess>,
    fd: i32,
    fdstat: i32,
) -> impl Future<Output = Result<i32>> + 'a {
    async move {
        let mut alloc = memory.get_alloc()?;

        let ret = alloc
            .with_write_to_guest(fdstat, Layout::new::<ffi::Fdstat>(), |alloc, fdstat| {
                wasi_snapshot_preview1::fd_fdstat_get(ctx, alloc.as_memory(), fd, fdstat)
            })
            .await?;

        alloc.finalise()?;

        Ok(ret)
    }
}

pub fn fd_prestat_get<'a>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemoryAdapter<impl DisjointGuestMemoryAccess>,
    fd: i32,
    prestat: i32,
) -> impl Future<Output = Result<i32>> + 'a {
    async move {
        let mut alloc = memory.get_alloc()?;

        let ret = alloc
            .with_write_to_guest(prestat, Layout::new::<ffi::Prestat>(), |alloc, prestat| {
                wasi_snapshot_preview1::fd_prestat_get(ctx, alloc.as_memory(), fd, prestat)
            })
            .await?;

        alloc.finalise()?;

        Ok(ret)
    }
}

pub fn fd_prestat_dir_name<'a>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemoryAdapter<impl DisjointGuestMemoryAccess>,
    fd: i32,
    path: i32,
    path_len: i32,
) -> impl Future<Output = Result<i32>> + 'a {
    async move {
        let mut alloc = memory.get_alloc()?;

        let path_len_usize = usize::try_from(u32::from_ne_bytes(path_len.to_ne_bytes()))?;
        let ret = alloc
            .with_write_to_guest(path, Layout::array::<u8>(path_len_usize)?, |alloc, path| {
                wasi_snapshot_preview1::fd_prestat_dir_name(
                    ctx,
                    alloc.as_memory(),
                    fd,
                    path,
                    path_len,
                )
            })
            .await?;

        alloc.finalise()?;

        Ok(ret)
    }
}

pub fn fd_read<'a>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemoryAdapter<impl DisjointGuestMemoryAccess>,
    fd: i32,
    iovs: i32,
    iovs_len: i32,
    nread: i32,
) -> impl Future<Output = Result<i32>> + 'a {
    async move {
        let mut alloc = memory.get_alloc()?;

        let ret = alloc
            .with_iovs_to_guest(iovs, iovs_len, |alloc, iovs| {
                alloc.with_write_to_guest(nread, Layout::new::<ffi::Size>(), move |alloc, nread| {
                    wasi_snapshot_preview1::fd_read(
                        ctx,
                        alloc.as_memory(),
                        fd,
                        iovs,
                        iovs_len,
                        nread,
                    )
                })
            })
            .await?;

        alloc.finalise()?;

        Ok(ret)
    }
}

pub fn fd_seek<'a>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemoryAdapter<impl DisjointGuestMemoryAccess>,
    fd: i32,
    offset: i64,
    whence: i32,
    new_offset: i32,
) -> impl Future<Output = Result<i32>> + 'a {
    async move {
        let mut alloc = memory.get_alloc()?;

        let ret = alloc
            .with_write_to_guest(
                new_offset,
                Layout::new::<ffi::Filesize>(),
                |alloc, new_offset| {
                    wasi_snapshot_preview1::fd_seek(
                        ctx,
                        alloc.as_memory(),
                        fd,
                        offset,
                        whence,
                        new_offset,
                    )
                },
            )
            .await?;

        alloc.finalise()?;

        Ok(ret)
    }
}

pub fn fd_write<'a>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemoryAdapter<impl DisjointGuestMemoryAccess>,
    fd: i32,
    ciovs: i32,
    ciovs_len: i32,
    nwritten: i32,
) -> impl Future<Output = Result<i32>> + 'a {
    async move {
        let mut alloc = memory.get_alloc()?;

        let ret = alloc
            .with_ciovs_from_guest(ciovs, ciovs_len, |alloc, ciovs| {
                alloc.with_write_to_guest(
                    nwritten,
                    Layout::new::<ffi::Size>(),
                    move |alloc, nwritten| {
                        wasi_snapshot_preview1::fd_write(
                            ctx,
                            alloc.as_memory(),
                            fd,
                            ciovs,
                            ciovs_len,
                            nwritten,
                        )
                    },
                )
            })
            .await?;

        alloc.finalise()?;

        Ok(ret)
    }
}

pub fn proc_exit<'a>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemoryAdapter<impl DisjointGuestMemoryAccess>,
    rval: i32,
) -> impl Future<Output = Result<()>> + 'a {
    async move {
        let mut alloc = memory.get_alloc()?;

        #[allow(clippy::let_unit_value)]
        let () = wasi_snapshot_preview1::proc_exit(ctx, alloc.as_memory(), rval).await?;

        alloc.finalise()?;

        Ok(())
    }
}
