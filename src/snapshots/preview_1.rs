use std::alloc::Layout;

use anyhow::Result;
use wasi_common::snapshots::preview_1::wasi_snapshot_preview1::{self, WasiSnapshotPreview1};

use crate::{DisjointGuestMemory, DisjointGuestMemoryAccess};

pub async fn clock_time_get<'a, A: DisjointGuestMemoryAccess>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemory<A>,
    clock_id: i32,
    precision: i64,
    time: i32,
) -> Result<i32> {
    let ret = memory
        .with_write_to_async(time, Layout::new::<wasi::Timestamp>(), |time| {
            wasi_snapshot_preview1::clock_time_get(ctx, memory, clock_id, precision, time)
        })
        .await?;
    memory.clear();
    Ok(ret)
}

pub async fn environ_get<'a, A: DisjointGuestMemoryAccess>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemory<A>,
    environ: i32,
    environ_buf: i32,
) -> Result<i32> {
    let (environ_size, environ_buf_size) = ctx.environ_sizes_get().await?;
    let ret = memory
        .with_write_to_async(
            environ,
            Layout::array::<u32>(usize::try_from(environ_size)?)?,
            |environ| async move {
                let ret = memory
                    .with_write_to_async(
                        environ_buf,
                        Layout::array::<u8>(usize::try_from(environ_buf_size)?)?,
                        |environ_buf| {
                            wasi_snapshot_preview1::environ_get(ctx, memory, environ, environ_buf)
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

pub async fn environ_sizes_get<'a, A: DisjointGuestMemoryAccess>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemory<A>,
    environ_size: i32,
    environ_buf_size: i32,
) -> Result<i32> {
    let ret = memory
        .with_write_to_async(environ_size, Layout::new::<wasi::Size>(), |environ_size| {
            memory.with_write_to_async(
                environ_buf_size,
                Layout::new::<wasi::Size>(),
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

pub async fn fd_close<'a, A: DisjointGuestMemoryAccess>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemory<A>,
    fd: i32,
) -> Result<i32> {
    wasi_snapshot_preview1::fd_close(ctx, memory, fd).await
}

pub async fn fd_fdstat_get<'a, A: DisjointGuestMemoryAccess>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemory<A>,
    fd: i32,
    fdstat: i32,
) -> Result<i32> {
    let ret = memory
        .with_write_to_async(fdstat, Layout::new::<wasi::Fdstat>(), |fdstat| {
            wasi_snapshot_preview1::fd_fdstat_get(ctx, memory, fd, fdstat)
        })
        .await?;
    memory.clear();
    Ok(ret)
}

pub async fn fd_prestat_get<'a, A: DisjointGuestMemoryAccess>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemory<A>,
    fd: i32,
    prestat: i32,
) -> Result<i32> {
    let ret = memory
        .with_write_to_async(prestat, Layout::new::<wasi::Prestat>(), |prestat| {
            wasi_snapshot_preview1::fd_prestat_get(ctx, memory, fd, prestat)
        })
        .await?;
    memory.clear();
    Ok(ret)
}

pub async fn fd_prestat_dir_name<'a, A: DisjointGuestMemoryAccess>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemory<A>,
    fd: i32,
    path: i32,
    path_len: i32,
) -> Result<i32> {
    let path_len_usize = usize::try_from(u32::from_ne_bytes(path_len.to_ne_bytes()))?;
    let ret = memory
        .with_write_to_async(path, Layout::array::<u8>(path_len_usize)?, |path| {
            wasi_snapshot_preview1::fd_prestat_dir_name(ctx, memory, fd, path, path_len)
        })
        .await?;
    memory.clear();
    Ok(ret)
}

// TODO: fd_read

pub async fn fd_seek<'a, A: DisjointGuestMemoryAccess>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemory<A>,
    fd: i32,
    offset: i64,
    whence: i32,
    new_offset: i32,
) -> Result<i32> {
    let ret = memory
        .with_write_to_async(new_offset, Layout::new::<wasi::Filesize>(), |new_offset| {
            wasi_snapshot_preview1::fd_seek(ctx, memory, fd, offset, whence, new_offset)
        })
        .await?;
    memory.clear();
    Ok(ret)
}

// TODO: fd_write

pub async fn proc_exit<'a, A: DisjointGuestMemoryAccess>(
    ctx: &'a mut impl WasiSnapshotPreview1,
    memory: &'a DisjointGuestMemory<A>,
    rval: i32,
) -> Result<()> {
    wasi_snapshot_preview1::proc_exit(ctx, memory, rval).await
}
