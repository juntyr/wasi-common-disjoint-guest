mod disjoint;
mod ffi;
pub mod snapshots;

pub use disjoint::{
    DisjointGuestMemory, DisjointGuestMemoryAccess, DisjointGuestMemoryAdapter,
    DisjointGuestMemoryAlloc, DisjointGuestMemoryAllocGuard,
};
