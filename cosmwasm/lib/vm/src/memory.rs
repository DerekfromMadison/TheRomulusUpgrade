use crate::errors::{Error, RegionTooSmallErr};
use wasmer_runtime_core::{
    memory::ptr::{Array, WasmPtr},
    types::ValueType,
    vm::Ctx,
};

/****** read/write to wasm memory buffer ****/

/// Refers to some heap allocated data in wasm.
/// A pointer to this can be returned over ffi boundaries.
///
/// This is the same as cosmwasm::memory::Region
/// but defined here to allow wasm impl
#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct Region {
    pub offset: u32,
    pub len: u32,
}

unsafe impl ValueType for Region {}

// Expects a (fixed size) Region struct at ptr, which is read. This links to the
// memory region, which is read in the second step.
pub fn read_region(ctx: &Ctx, ptr: u32) -> Vec<u8> {
    let region = to_region(ctx, ptr);
    let memory = ctx.memory(0);

    match WasmPtr::<u8, Array>::new(region.offset).deref(memory, 0, region.len) {
        Some(cells) => {
            // In case you want to do some premature optimization, this shows how to cast a `&'mut [Cell<u8>]` to `&mut [u8]`:
            // https://github.com/wasmerio/wasmer/blob/0.13.1/lib/wasi/src/syscalls/mod.rs#L79-L81
            let len = region.len as usize;
            let mut result = vec![0u8; len];
            for i in 0..len {
                result[i] = cells[i].get();
            }
            result
        }
        None => panic!(
            "Error dereferencing region {:?} in wasm memory of size {}. This typically happens when the given pointer does not point to a Region struct.",
            region,
            memory.size().bytes().0
        ),
    }
}

/// A prepared and sufficiently large memory Region is expected at ptr that points to pre-allocated memory.
///
/// Returns number of bytes written on success.
pub fn write_region(ctx: &Ctx, ptr: u32, data: &[u8]) -> Result<usize, Error> {
    let region = to_region(ctx, ptr);
    let region_size = region.len as usize;

    if data.len() > region_size {
        return RegionTooSmallErr {
            size: region_size,
            required: data.len(),
        }
        .fail();
    }

    // A performance optimization
    if data.is_empty() {
        return Ok(0);
    }

    let memory = ctx.memory(0);

    match WasmPtr::<u8, Array>::new(region.offset).deref(memory, 0, region.len) {
        Some(cells) => {
            // In case you want to do some premature optimization, this shows how to cast a `&'mut [Cell<u8>]` to `&mut [u8]`:
            // https://github.com/wasmerio/wasmer/blob/0.13.1/lib/wasi/src/syscalls/mod.rs#L79-L81
            for i in 0..data.len() {
                cells[i].set(data[i])
            }
            Ok(data.len())
        },
        None => panic!(
            "Error dereferencing region {:?} in wasm memory of size {}. This typically happens when the given pointer does not point to a Region struct.",
            region,
            memory.size().bytes().0
        ),
    }
}

// Reads in a ptr to Region in wasm memory and constructs the object we can use to access it
fn to_region(ctx: &Ctx, ptr: u32) -> Region {
    let memory = ctx.memory(0);
    let wptr = WasmPtr::<Region>::new(ptr);
    let cell = wptr.deref(memory).unwrap();
    cell.get()
}
