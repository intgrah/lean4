//! FFI to Lean's compacted-region API (`src/library/module.cpp`,
//! `src/runtime/compact.cpp`).
//!
//! Corpus payloads are bytes written by `Lean.CompactedRegion.save`. To
//! consume them on the Rust side we go back through Lean's runtime: write
//! the bytes to a temp file, call `lean_compacted_region_read` to mmap
//! them, and hand back the root `*mut lean_object` and a [`Region`] handle
//! that releases the mapping on drop.
//!
//! The C symbols here aren't in `src/include/lean/lean.h` (they're
//! Lean-internal, exposed only through `@[extern]` opaques in
//! `src/Lean/CompactedRegion.lean`), so they live in this hand-written
//! module rather than the auto-generated `lean-runtime-sys` bindings.
//!
//! ## Linkage
//!
//! These symbols are defined in `libleanshared.so`. Binaries that use this
//! module (notably `lean-diff`) must link against it; see the
//! `lean-diff` crate's `build.rs` (forthcoming).

use core::ffi::c_void;

use lean_runtime_sys::lean_object;

/// Opaque CompactedRegion handle. Matches Lean's `def CompactedRegion := USize`,
/// which at the C ABI is a raw pointer to a `compacted_region` C++ object
/// (the `USize` reinterprets the pointer).
pub type LeanCompactedRegion = usize;

unsafe extern "C" {
    /// Returns true if the region was loaded via mmap (vs. copied into a
    /// heap buffer). Always succeeds; no IO failure path.
    pub fn lean_compacted_region_is_memory_mapped(region: LeanCompactedRegion) -> u8;

    /// Region size in bytes.
    pub fn lean_compacted_region_size(region: LeanCompactedRegion) -> usize;

    /// Releases the region. Lean's signature is `IO Unit`, which at the C
    /// ABI returns a boxed IO result that the caller must unwrap and free.
    /// The trailing `*mut lean_object` is the `RealWorld` token Lean
    /// threads through IO.
    pub fn lean_compacted_region_free(
        region: LeanCompactedRegion,
        world: *mut lean_object,
    ) -> *mut lean_object;

    /// Reads a compacted region from `fname` (a Lean `String`) using
    /// `dep_regions` (a Lean `Array CompactedRegion`) for cross-region
    /// pointer fixup. Returns an `IO (α × CompactedRegion)` wrapped in a
    /// Lean object; the caller must unwrap the IO result and extract the
    /// `(root, region)` tuple. `α` is type-erased at the boundary.
    pub fn lean_compacted_region_read(
        fname: *mut lean_object,
        dep_regions: *mut lean_object,
        world: *mut lean_object,
    ) -> *mut lean_object;
}

/// Owned compacted-region handle. Drops via `lean_compacted_region_free`.
///
/// Construction is not yet wired here; the `load_from_bytes` helper lives
/// in `lean-diff` because it needs the IO-unwrap shim that depends on
/// `lean-runtime-sys` helpers we haven't yet exercised.
#[repr(transparent)]
pub struct Region {
    handle: LeanCompactedRegion,
}

impl Region {
    /// Wrap a raw handle. The caller transfers ownership: the [`Region`]
    /// will free it on drop.
    ///
    /// # Safety
    /// `handle` must be a valid compacted-region handle obtained from
    /// `lean_compacted_region_read` (or equivalent) and not yet freed.
    #[inline]
    pub unsafe fn from_raw(handle: LeanCompactedRegion) -> Self {
        Self { handle }
    }

    /// The raw C++ `compacted_region *` reinterpreted as `usize`.
    #[inline]
    pub fn handle(&self) -> LeanCompactedRegion {
        self.handle
    }

    /// Region size in bytes (forwards to `lean_compacted_region_size`).
    pub fn size(&self) -> usize {
        unsafe { lean_compacted_region_size(self.handle) }
    }

    /// True iff the region's backing memory is an mmap of the on-disk file.
    pub fn is_memory_mapped(&self) -> bool {
        unsafe { lean_compacted_region_is_memory_mapped(self.handle) != 0 }
    }

    /// Release the region without invoking the destructor. The caller
    /// becomes responsible for calling `lean_compacted_region_free`.
    pub fn into_raw(self) -> LeanCompactedRegion {
        let h = self.handle;
        core::mem::forget(self);
        h
    }
}

impl Drop for Region {
    fn drop(&mut self) {
        // SAFETY: `world` is a token in Lean's IO threading. lean_box(0) is
        // the canonical Unit/RealWorld stand-in used by Lean-emitted C code
        // when invoking IO-returning externs from non-IO contexts. The
        // returned IO result is leaked: it carries either Unit (success) or
        // an error payload, neither of which we surface from Drop. A
        // future iteration should unwrap-and-log; for now we treat
        // region-free failures as unrecoverable and ignore.
        let _ = core::mem::size_of::<c_void>();
        unsafe {
            let world = lean_runtime_sys::lean_box(0);
            let _io_result = lean_compacted_region_free(self.handle, world);
        }
    }
}
