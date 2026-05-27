use core::ffi::c_void;

use lean_runtime_sys::lean_object;

pub type LeanCompactedRegion = usize;

unsafe extern "C" {
    pub fn lean_compacted_region_is_memory_mapped(region: LeanCompactedRegion) -> u8;

    pub fn lean_compacted_region_size(region: LeanCompactedRegion) -> usize;

    pub fn lean_compacted_region_free(
        region: LeanCompactedRegion,
        world: *mut lean_object,
    ) -> *mut lean_object;

    pub fn lean_compacted_region_read(
        fname: *mut lean_object,
        dep_regions: *mut lean_object,
        world: *mut lean_object,
    ) -> *mut lean_object;
}

#[repr(transparent)]
pub struct Region {
    handle: LeanCompactedRegion,
}

impl Region {
    #[inline]
    pub unsafe fn from_raw(handle: LeanCompactedRegion) -> Self {
        Self { handle }
    }

    #[inline]
    pub fn handle(&self) -> LeanCompactedRegion {
        self.handle
    }

    pub fn size(&self) -> usize {
        unsafe { lean_compacted_region_size(self.handle) }
    }

    pub fn is_memory_mapped(&self) -> bool {
        unsafe { lean_compacted_region_is_memory_mapped(self.handle) != 0 }
    }

    pub fn into_raw(self) -> LeanCompactedRegion {
        let h = self.handle;
        core::mem::forget(self);
        h
    }
}

impl Drop for Region {
    fn drop(&mut self) {
        let _ = core::mem::size_of::<c_void>();
        unsafe {
            let world = lean_runtime_sys::lean_box(0);
            let _io_result = lean_compacted_region_free(self.handle, world);
        }
    }
}
