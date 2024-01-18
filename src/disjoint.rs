use std::{
    alloc::Layout,
    cell::UnsafeCell,
    future::Future,
    ops::{Deref, DerefMut},
    sync::{Mutex, MutexGuard},
};

use wiggle::{borrow::BorrowChecker, GuestMemory};

use crate::ffi;

pub trait DisjointGuestMemoryAccess: Send + Sync {
    fn read(&self, offset: u32, into: &mut [u8]) -> Result<(), anyhow::Error>;
    fn write(&self, offset: u32, from: &[u8]) -> Result<(), anyhow::Error>;
}

pub struct DisjointGuestMemoryAdapter<A: DisjointGuestMemoryAccess> {
    inner: Mutex<DisjointGuestMemoryAlloc<A>>,
}

pub struct DisjointGuestMemoryAlloc<A: DisjointGuestMemoryAccess> {
    access: A,
    buffer: Vec<UnsafeCell<u8>>,
    writes: Vec<DelayedWrite>,
    bc: BorrowChecker,
}

unsafe impl<A: DisjointGuestMemoryAccess> Sync for DisjointGuestMemoryAlloc<A> {}

enum DelayedWrite {
    Write {
        guest_into: i32,
        host_from: i32,
        layout: Layout,
    },
    Iovs {
        guest_into: i32,
        host_from: i32,
        len: usize,
    },
}

pub struct DisjointGuestMemoryAllocGuard<'a, A: DisjointGuestMemoryAccess> {
    guard: MutexGuard<'a, DisjointGuestMemoryAlloc<A>>,
}

impl<'a, A: DisjointGuestMemoryAccess> DisjointGuestMemoryAllocGuard<'a, A> {
    pub fn finalise(mut self) -> anyhow::Result<()> {
        self.finalise_inner()
    }

    fn finalise_inner(&mut self) -> anyhow::Result<()> {
        let mut writes = std::mem::take(&mut self.writes);
        for write in writes.drain(..) {
            match write {
                DelayedWrite::Write {
                    guest_into,
                    host_from,
                    layout,
                } => self.write(host_from, guest_into, layout.size())?,
                DelayedWrite::Iovs {
                    guest_into: iovs,
                    host_from: host_iovs,
                    len: iovs_len,
                } => {
                    let iovec_layout = Layout::new::<ffi::Iovec>();
                    let size_layout = Layout::new::<ffi::Size>();

                    let iovs = u32::from_ne_bytes(iovs.to_ne_bytes());
                    let host_iovs_usize =
                        usize::try_from(u32::from_ne_bytes(host_iovs.to_ne_bytes()))?;

                    for i in 0..iovs_len {
                        let buf: ffi::Pointer = 0;
                        let mut buf = buf.to_le_bytes();
                        buf.copy_from_slice(
                            &self.get_buffer_mut()[(host_iovs_usize + i * iovec_layout.size())
                                ..(host_iovs_usize + i * iovec_layout.size() + size_layout.size())],
                        );
                        let buf = ffi::Pointer::from_le_bytes(buf);
                        let buf = i32::from_ne_bytes(buf.to_ne_bytes());

                        let len: ffi::Size = 0;
                        let mut len = len.to_le_bytes();
                        len.copy_from_slice(
                            &self.get_buffer_mut()[(host_iovs_usize
                                + i * iovec_layout.size()
                                + size_layout.size())
                                ..(host_iovs_usize + (i + 1) * iovec_layout.size())],
                        );
                        let len = ffi::Size::from_le_bytes(len);
                        let len = usize::try_from(len)?;

                        let ptr: ffi::Pointer = 0;
                        let mut ptr: [u8; 4] = ptr.to_le_bytes();
                        self.access
                            .read(iovs + u32::try_from(i * iovec_layout.size())?, &mut ptr)?;
                        let ptr = ffi::Pointer::from_le_bytes(ptr);
                        let ptr = i32::from_ne_bytes(ptr.to_ne_bytes());

                        self.write(buf, ptr, len)?;
                    }
                }
            }
        }
        self.writes = writes;

        self.buffer.clear();

        Ok(())
    }
}

impl<'a, A: DisjointGuestMemoryAccess> Deref for DisjointGuestMemoryAllocGuard<'a, A> {
    type Target = DisjointGuestMemoryAlloc<A>;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<'a, A: DisjointGuestMemoryAccess> DerefMut for DisjointGuestMemoryAllocGuard<'a, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

impl<'a, A: DisjointGuestMemoryAccess> Drop for DisjointGuestMemoryAllocGuard<'a, A> {
    fn drop(&mut self) {
        self.finalise_inner().unwrap();
    }
}

#[repr(transparent)]
pub struct DisjointGuestMemory<A: DisjointGuestMemoryAccess> {
    inner: DisjointGuestMemoryAlloc<A>,
}

unsafe impl<A: DisjointGuestMemoryAccess> GuestMemory for DisjointGuestMemory<A> {
    fn base(&self) -> &[UnsafeCell<u8>] {
        &self.inner.buffer
    }

    fn has_outstanding_borrows(&self) -> bool {
        self.inner.bc.has_outstanding_borrows()
    }

    fn is_mut_borrowed(&self, r: wiggle::Region) -> bool {
        self.inner.bc.is_mut_borrowed(r)
    }

    fn is_shared_borrowed(&self, r: wiggle::Region) -> bool {
        self.inner.bc.is_shared_borrowed(r)
    }

    fn mut_borrow(&self, r: wiggle::Region) -> Result<wiggle::BorrowHandle, wiggle::GuestError> {
        self.inner.bc.mut_borrow(r)
    }

    fn shared_borrow(&self, r: wiggle::Region) -> Result<wiggle::BorrowHandle, wiggle::GuestError> {
        self.inner.bc.shared_borrow(r)
    }

    fn mut_unborrow(&self, h: wiggle::BorrowHandle) {
        self.inner.bc.mut_unborrow(h)
    }

    fn shared_unborrow(&self, h: wiggle::BorrowHandle) {
        self.inner.bc.shared_unborrow(h)
    }
}

impl<A: DisjointGuestMemoryAccess> DisjointGuestMemoryAdapter<A> {
    #[must_use]
    pub fn new(access: A) -> Self {
        Self {
            inner: Mutex::new(DisjointGuestMemoryAlloc {
                access,
                buffer: Vec::new(),
                writes: Vec::new(),
                bc: BorrowChecker::new(),
            }),
        }
    }

    pub fn get_alloc(&self) -> anyhow::Result<DisjointGuestMemoryAllocGuard<A>> {
        match self.inner.lock() {
            Ok(guard) => Ok(DisjointGuestMemoryAllocGuard { guard }),
            Err(_) => Err(anyhow::format_err!(
                "DisjointGuestMemoryAdapter is poisoned"
            )),
        }
    }
}

impl<A: DisjointGuestMemoryAccess> DisjointGuestMemoryAlloc<A> {
    pub fn get_memory(&mut self) -> &DisjointGuestMemory<A> {
        // Safety: DisjointGuestMemory is a transparent newtype around Self
        // Note: Taking a mutable borrow also ensures that the interior
        //       mutability GuestMemory::base API is only exposed within an
        //       exterior mutability scope
        unsafe { &*(self as *const Self).cast() }
    }

    pub async fn with_write_to_guest<
        'a,
        T,
        E: Into<anyhow::Error>,
        F: Future<Output = Result<T, E>> + 'a,
    >(
        &'a mut self,
        guest: i32,
        layout: Layout,
        inner: impl FnOnce(&'a mut Self, i32) -> F,
    ) -> Result<T, anyhow::Error> {
        // TODO: optimise
        let host = self.alloc(layout);

        self.writes.push(DelayedWrite::Write {
            guest_into: guest,
            host_from: host,
            layout,
        });

        inner(self, host).await.map_err(Into::into)
    }

    pub async fn with_iovs_to_guest<
        'a,
        T,
        E: Into<anyhow::Error>,
        F: Future<Output = Result<T, E>>,
    >(
        &'a mut self,
        iovs: i32,
        iovs_len: i32,
        inner: impl FnOnce(&'a mut Self, i32) -> F,
    ) -> Result<T, anyhow::Error> {
        // TODO: optimise
        let iovs = u32::from_ne_bytes(iovs.to_ne_bytes());
        let iovs_len = usize::try_from(u32::from_ne_bytes(iovs_len.to_ne_bytes()))?;

        let iovec_layout = Layout::new::<ffi::Iovec>();
        let size_layout = Layout::new::<ffi::Size>();

        let host_iovs = self.alloc(Layout::array::<ffi::Iovec>(iovs_len)?);
        let host_iovs_usize = usize::try_from(u32::from_ne_bytes(host_iovs.to_ne_bytes()))?;

        for i in 0..iovs_len {
            // TODO: check endianness
            let len: ffi::Size = 0;
            let mut len = len.to_le_bytes();
            self.access
                .read(iovs + u32::try_from(size_layout.size())?, &mut len)?;
            let len = ffi::Size::from_le_bytes(len);
            let len_usize = usize::try_from(len)?;

            let host_iov = self.alloc(Layout::array::<u8>(len_usize)?);
            self.get_buffer_mut()[(host_iovs_usize + i * iovec_layout.size())
                ..(host_iovs_usize + i * iovec_layout.size() + size_layout.size())]
                .copy_from_slice(&host_iov.to_le_bytes());
            self.get_buffer_mut()[(host_iovs_usize + i * iovec_layout.size() + size_layout.size())
                ..(host_iovs_usize + (i + 1) * iovec_layout.size())]
                .copy_from_slice(&len.to_le_bytes());
        }

        let iovs = i32::from_ne_bytes(iovs.to_ne_bytes());

        self.writes.push(DelayedWrite::Iovs {
            guest_into: iovs,
            host_from: host_iovs,
            len: iovs_len,
        });

        inner(self, host_iovs).await.map_err(Into::into)
    }

    pub async fn with_ciovs_from_guest<
        'a,
        T,
        E: Into<anyhow::Error>,
        F: Future<Output = Result<T, E>>,
    >(
        &'a mut self,
        ciovs: i32,
        ciovs_len: i32,
        inner: impl FnOnce(&'a mut Self, i32) -> F,
    ) -> Result<T, anyhow::Error> {
        // TODO: optimise
        let ciovs = u32::from_ne_bytes(ciovs.to_ne_bytes());
        let ciovs_len = usize::try_from(u32::from_ne_bytes(ciovs_len.to_ne_bytes()))?;

        let iovec_layout = Layout::new::<ffi::Iovec>();
        let size_layout = Layout::new::<ffi::Size>();

        let host_ciovs = self.alloc(Layout::array::<ffi::Iovec>(ciovs_len)?);
        let host_ciovs_usize = usize::try_from(u32::from_ne_bytes(host_ciovs.to_ne_bytes()))?;

        for i in 0..ciovs_len {
            // TODO: check endianness
            let ptr: ffi::Pointer = 0;
            let mut ptr = ptr.to_le_bytes();
            self.access.read(ciovs, &mut ptr)?;
            let ptr = ffi::Pointer::from_le_bytes(ptr);
            let ptr = i32::from_ne_bytes(ptr.to_ne_bytes());

            let len: ffi::Size = 0;
            let mut len = len.to_le_bytes();
            self.access
                .read(ciovs + u32::try_from(size_layout.size())?, &mut len)?;
            let len = ffi::Size::from_le_bytes(len);
            let len = usize::try_from(len)?;

            let host_ciov = self.alloc(Layout::array::<u8>(len)?);
            self.read(ptr, host_ciov, len)?;

            self.get_buffer_mut()[(host_ciovs_usize + i * iovec_layout.size())
                ..(host_ciovs_usize + i * iovec_layout.size() + size_layout.size())]
                .copy_from_slice(&host_ciov.to_le_bytes());
            self.get_buffer_mut()[(host_ciovs_usize + i * iovec_layout.size() + size_layout.size())
                ..(host_ciovs_usize + (i + 1) * iovec_layout.size())]
                .copy_from_slice(&len.to_le_bytes());
        }

        inner(self, host_ciovs).await.map_err(Into::into)
    }
}

impl<A: DisjointGuestMemoryAccess> DisjointGuestMemoryAlloc<A> {
    fn get_buffer_mut(&mut self) -> &mut [u8] {
        retag_mut(&mut self.buffer)
    }

    fn alloc(&mut self, layout: Layout) -> i32 {
        let ptr = self
            .buffer
            .len()
            .wrapping_add(layout.align())
            .wrapping_sub(1)
            & !layout.align().wrapping_sub(1);
        let padding = ptr.wrapping_sub(self.buffer.len());

        self.buffer.reserve(padding + layout.size());

        for _ in 0..(padding + layout.size()) {
            self.buffer.push(UnsafeCell::new(0));
        }

        i32::from_ne_bytes((ptr as u32).to_ne_bytes())
    }

    fn read(&mut self, guest_from: i32, host_into: i32, len: usize) -> Result<(), anyhow::Error> {
        let guest_from = u32::from_ne_bytes(guest_from.to_ne_bytes());
        let host_into = u32::from_ne_bytes(host_into.to_ne_bytes());

        let len_u32 = u32::try_from(len)?;

        let host_region = wiggle::Region {
            start: host_into,
            len: len_u32,
        };

        let host_into = usize::try_from(host_into)?;
        let host = self
            .buffer
            .get_mut(host_into..)
            .and_then(|s| s.get_mut(0..len))
            .ok_or(wiggle::GuestError::PtrOutOfBounds(host_region))?;
        // Note: we need to use retag directly to be explicit about disjoint
        //       mutable borrows over self.buffer and self.access
        let host: &mut [u8] = retag_mut(host);

        self.access.read(guest_from, host)?;

        Ok(())
    }

    fn write(&mut self, host_from: i32, guest_into: i32, len: usize) -> Result<(), anyhow::Error> {
        let host_from = u32::from_ne_bytes(host_from.to_ne_bytes());
        let guest_into = u32::from_ne_bytes(guest_into.to_ne_bytes());

        let len_u32 = u32::try_from(len)?;

        let host_region: wiggle::Region = wiggle::Region {
            start: host_from,
            len: len_u32,
        };

        let host_from = usize::try_from(host_from)?;
        let host = self
            .buffer
            .get_mut(host_from..)
            .and_then(|s| s.get_mut(0..len))
            .ok_or(wiggle::GuestError::PtrOutOfBounds(host_region))?;
        // Note: we need to use retag directly to be explicit about disjoint
        //       mutable borrows over self.buffer and self.access
        let host: &[u8] = retag_mut(host);

        self.access.write(guest_into, host)?;

        Ok(())
    }
}

fn retag_mut(v: &mut [UnsafeCell<u8>]) -> &mut [u8] {
    // Safety:
    //  - &mut [UnsafeCell<u8>] gives us access to each &mut UnsafeCell<u8>
    //  - UnsafeCell<u8>::get_mut turns &mut UnsafeCell<u8> into &mut u8
    unsafe { std::mem::transmute(v) }
}
