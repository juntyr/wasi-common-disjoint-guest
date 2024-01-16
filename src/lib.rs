use std::{
    alloc::Layout,
    cell::{RefCell, UnsafeCell},
    future::Future,
};

use wiggle::{borrow::BorrowChecker, GuestMemory};

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
            buffer.push(0.into());
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

    async fn with_write_to_async<T, E: Into<anyhow::Error>, F: Future<Output = Result<T, E>>>(
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
}
