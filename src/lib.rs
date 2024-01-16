use std::{
    alloc::Layout,
    cell::{RefCell, UnsafeCell},
    future::Future,
};

use wiggle::{borrow::BorrowChecker, GuestMemory};

mod ffi;
pub mod snapshots;

pub trait DisjointGuestMemoryAccess: Send + Sync {
    fn read(&self, offset: u32, into: &mut [u8]) -> Result<(), wiggle::GuestError>;
    fn write(&self, offset: u32, from: &[u8]) -> Result<(), wiggle::GuestError>;
}

pub struct DisjointGuestMemory<A: DisjointGuestMemoryAccess> {
    access: A,
    buffer: RefCell<Vec<u8>>,
    bc: BorrowChecker,
}

unsafe impl<A: DisjointGuestMemoryAccess> Send for DisjointGuestMemory<A> {}
unsafe impl<A: DisjointGuestMemoryAccess> Sync for DisjointGuestMemory<A> {}

unsafe impl<A: DisjointGuestMemoryAccess> GuestMemory for DisjointGuestMemory<A> {
    fn base(&self) -> &[UnsafeCell<u8>] {
        // TODO: error + unsafety handling
        let buffer = self.buffer.borrow();
        unsafe { std::slice::from_raw_parts(buffer.as_ptr().cast(), buffer.len()) }
    }

    fn has_outstanding_borrows(&self) -> bool {
        self.bc.has_outstanding_borrows()
    }

    fn is_mut_borrowed(&self, r: wiggle::Region) -> bool {
        self.bc.is_mut_borrowed(r)
    }

    fn is_shared_borrowed(&self, r: wiggle::Region) -> bool {
        self.bc.is_shared_borrowed(r)
    }

    fn mut_borrow(&self, r: wiggle::Region) -> Result<wiggle::BorrowHandle, wiggle::GuestError> {
        self.bc.mut_borrow(r)
    }

    fn shared_borrow(&self, r: wiggle::Region) -> Result<wiggle::BorrowHandle, wiggle::GuestError> {
        self.bc.shared_borrow(r)
    }

    fn mut_unborrow(&self, h: wiggle::BorrowHandle) {
        self.bc.mut_unborrow(h)
    }

    fn shared_unborrow(&self, h: wiggle::BorrowHandle) {
        self.bc.shared_unborrow(h)
    }
}

impl<A: DisjointGuestMemoryAccess> DisjointGuestMemory<A> {
    // TODO: use guard-based API instead
    fn alloc(&self, layout: Layout) -> i32 {
        let mut buffer = self.buffer.borrow_mut();

        let ptr = buffer.len().wrapping_add(layout.align()).wrapping_sub(1)
            & !layout.align().wrapping_sub(1);
        let padding = ptr.wrapping_sub(buffer.len());

        buffer.reserve(padding + layout.size());

        for _ in 0..(padding + layout.size()) {
            buffer.push(0);
        }

        i32::from_ne_bytes((ptr as u32).to_ne_bytes())
    }

    fn clear(&self) {
        self.buffer.borrow_mut().clear();
    }

    fn read(&self, guest_from: i32, host_into: i32, len: usize) -> Result<(), wiggle::GuestError> {
        let mut buffer = self.buffer.borrow_mut();

        let guest_from = u32::from_ne_bytes(guest_from.to_ne_bytes());
        let host_into = u32::from_ne_bytes(host_into.to_ne_bytes());

        let len_u32 = u32::try_from(len)?;

        let host_region = wiggle::Region {
            start: host_into,
            len: len_u32,
        };

        let host_into = usize::try_from(host_into)?;
        let host = buffer
            .get_mut(host_into..)
            .and_then(|s| s.get_mut(0..len))
            .ok_or(wiggle::GuestError::PtrOutOfBounds(host_region))?;

        self.access.read(guest_from, host)?;

        Ok(())
    }

    fn write(&self, host_from: i32, guest_into: i32, len: usize) -> Result<(), wiggle::GuestError> {
        let buffer = self.buffer.borrow();

        let host_from = u32::from_ne_bytes(host_from.to_ne_bytes());
        let guest_into = u32::from_ne_bytes(guest_into.to_ne_bytes());

        let len_u32 = u32::try_from(len)?;

        let host_region: wiggle::Region = wiggle::Region {
            start: host_from,
            len: len_u32,
        };

        let host_from = usize::try_from(host_from)?;
        let host = buffer
            .get(host_from..)
            .and_then(|s| s.get(0..len))
            .ok_or(wiggle::GuestError::PtrOutOfBounds(host_region))?;

        self.access.write(guest_into, host)?;

        Ok(())
    }

    async fn with_write_to_guest<T, E: Into<anyhow::Error>, F: Future<Output = Result<T, E>>>(
        &self,
        guest: i32,
        layout: Layout,
        inner: impl FnOnce(i32) -> F,
    ) -> Result<T, anyhow::Error> {
        // TODO: optimise
        let host = self.alloc(layout);

        let res = inner(host).await.map_err(Into::into)?;

        self.write(host, guest, layout.size())?;

        Ok(res)
    }

    async fn with_iovs_to_guest<T, E: Into<anyhow::Error>, F: Future<Output = Result<T, E>>>(
        &self,
        iovs: i32,
        iovs_len: i32,
        inner: impl FnOnce(i32) -> F,
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
            self.buffer.borrow_mut()[(host_iovs_usize + i * iovec_layout.size())
                ..(host_iovs_usize + i * iovec_layout.size() + size_layout.size())]
                .copy_from_slice(&host_iov.to_le_bytes());
            self.buffer.borrow_mut()[(host_iovs_usize
                + i * iovec_layout.size()
                + size_layout.size())
                ..(host_iovs_usize + (i + 1) * iovec_layout.size())]
                .copy_from_slice(&len.to_le_bytes());
        }

        let res = inner(host_iovs).await.map_err(Into::into)?;

        for i in 0..iovs_len {
            let buf: ffi::Pointer = 0;
            let mut buf = buf.to_le_bytes();
            buf.copy_from_slice(
                &self.buffer.borrow()[(host_iovs_usize + i * iovec_layout.size())
                    ..(host_iovs_usize + i * iovec_layout.size() + size_layout.size())],
            );
            let buf = ffi::Pointer::from_le_bytes(buf);
            let buf = i32::from_ne_bytes(buf.to_ne_bytes());

            let len: ffi::Size = 0;
            let mut len = len.to_le_bytes();
            len.copy_from_slice(
                &self.buffer.borrow_mut()[(host_iovs_usize
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

        Ok(res)
    }

    async fn with_ciovs_from_guest<T, E: Into<anyhow::Error>, F: Future<Output = Result<T, E>>>(
        &self,
        ciovs: i32,
        ciovs_len: i32,
        inner: impl FnOnce(i32) -> F,
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

            self.buffer.borrow_mut()[(host_ciovs_usize + i * iovec_layout.size())
                ..(host_ciovs_usize + i * iovec_layout.size() + size_layout.size())]
                .copy_from_slice(&host_ciov.to_le_bytes());
            self.buffer.borrow_mut()[(host_ciovs_usize
                + i * iovec_layout.size()
                + size_layout.size())
                ..(host_ciovs_usize + (i + 1) * iovec_layout.size())]
                .copy_from_slice(&len.to_le_bytes());
        }

        inner(host_ciovs).await.map_err(Into::into)
    }
}
