//! Custom memory pool allocation for high-performance contract execution
//!
//! This module provides memory pool allocators optimized for frequent contract operations,
//! reducing allocation overhead and improving cache locality.

use crate::error::{ContractError, ContractResult};
use alloc::alloc::{alloc, dealloc, Layout};
use alloc::vec::Vec;
use core::cell::{RefCell, SyncUnsafeCell};
use core::hint::spin_loop;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicU8, Ordering};

/// Memory pool allocator for frequent allocations
pub struct MemoryPool {
    pools: RefCell<[Vec<*mut u8>; 8]>, // Pools for different size classes
    size_classes: [usize; 8],         // Size classes: 16, 32, 64, 128, 256, 512, 1024, 2048
}

impl MemoryPool {
    /// Create a new memory pool
    pub fn new() -> Self {
        Self {
            pools: RefCell::new([
                Vec::new(), Vec::new(), Vec::new(), Vec::new(),
                Vec::new(), Vec::new(), Vec::new(), Vec::new(),
            ]),
            size_classes: [16, 32, 64, 128, 256, 512, 1024, 2048],
        }
    }

    /// Allocate memory from the pool
    pub fn allocate(&self, layout: Layout) -> ContractResult<*mut u8> {
        let size = layout.size();
        let align = layout.align();

        // Find appropriate size class
        let size_class_idx = self.find_size_class(size);
        if size_class_idx >= self.size_classes.len() {
            // Too large for pool, use global allocator
            return unsafe {
                let ptr = alloc(layout);
                if ptr.is_null() {
                    Err(ContractError::StorageWriteFailed)
                } else {
                    Ok(ptr)
                }
            };
        }

        let actual_size = self.size_classes[size_class_idx];
        if actual_size < size || align > actual_size {
            // Size or alignment doesn't fit, use global allocator
            return unsafe {
                let ptr = alloc(layout);
                if ptr.is_null() {
                    Err(ContractError::StorageWriteFailed)
                } else {
                    Ok(ptr)
                }
            };
        }

        // Try to get from pool
        let mut pools = self.pools.borrow_mut();
        if let Some(ptr) = pools[size_class_idx].pop() {
            return Ok(ptr);
        }

        // Allocate new block
        unsafe {
            let ptr = alloc(Layout::from_size_align(actual_size, align).unwrap());
            if ptr.is_null() {
                Err(ContractError::StorageWriteFailed)
            } else {
                Ok(ptr)
            }
        }
    }

    /// Deallocate memory back to the pool
    pub fn deallocate(&self, ptr: *mut u8, layout: Layout) {
        let size = layout.size();

        // Find appropriate size class
        let size_class_idx = self.find_size_class(size);
        if size_class_idx >= self.size_classes.len() {
            // Too large for pool, use global deallocator
            unsafe {
                dealloc(ptr, layout);
            }
            return;
        }

        // Return to pool
        let mut pools = self.pools.borrow_mut();
        pools[size_class_idx].push(ptr);
    }

    /// Find the appropriate size class for a given size
    #[inline(always)]
    fn find_size_class(&self, size: usize) -> usize {
        // Binary search for size class
        let mut low = 0;
        let mut high = self.size_classes.len();

        while low < high {
            let mid = low + (high - low) / 2;
            if self.size_classes[mid] < size {
                low = mid + 1;
            } else {
                high = mid;
            }
        }

        low
    }

    /// Get pool statistics
    pub fn stats(&self) -> MemoryStats {
        let pools = self.pools.borrow();
        let mut total_allocated = 0;
        let mut total_available = 0;

        for (i, pool) in pools.iter().enumerate() {
            let size = self.size_classes[i];
            total_allocated += pool.len() * size;
            total_available += pool.capacity() * size;
        }

        MemoryStats {
            total_allocated,
            total_available,
        }
    }
}

/// Memory pool statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_allocated: usize,
    pub total_available: usize,
}

const STATE_UNINITIALIZED: u8 = 0;
const STATE_INITIALIZING: u8 = 1;
const STATE_READY: u8 = 2;

static MEMORY_POOL_STATE: AtomicU8 = AtomicU8::new(STATE_UNINITIALIZED);
static MEMORY_POOL: SyncUnsafeCell<MaybeUninit<MemoryPool>> = SyncUnsafeCell::new(MaybeUninit::uninit());

/// Get the global memory pool
#[inline(always)]
pub fn memory_pool() -> &'static MemoryPool {
    loop {
        match MEMORY_POOL_STATE.load(Ordering::Acquire) {
            STATE_READY => {
                return unsafe { &*(*MEMORY_POOL.get()).as_ptr() };
            }
            STATE_UNINITIALIZED => {
                if MEMORY_POOL_STATE
                    .compare_exchange(STATE_UNINITIALIZED, STATE_INITIALIZING, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
                {
                    unsafe {
                        (*MEMORY_POOL.get()).write(MemoryPool::new());
                    }
                    MEMORY_POOL_STATE.store(STATE_READY, Ordering::Release);
                    return unsafe { &*(*MEMORY_POOL.get()).as_ptr() };
                }
            }
            _ => spin_loop(),
        }
    }
}

/// Arena allocator for temporary allocations within a contract call
pub struct Arena {
    allocations: RefCell<Vec<(*mut u8, Layout)>>,
    pool: &'static MemoryPool,
}

impl Arena {
    /// Create a new arena
    pub fn new() -> Self {
        Self {
            allocations: RefCell::new(Vec::new()),
            pool: memory_pool(),
        }
    }

    /// Allocate memory in the arena
    pub fn allocate(&self, layout: Layout) -> ContractResult<*mut u8> {
        let ptr = self.pool.allocate(layout)?;
        self.allocations.borrow_mut().push((ptr, layout));
        Ok(ptr)
    }

    /// Reset the arena (deallocate all memory)
    pub fn reset(&self) {
        let mut allocations = self.allocations.borrow_mut();
        for (ptr, layout) in allocations.drain(..) {
            self.pool.deallocate(ptr, layout);
        }
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        self.reset();
    }
}

/// Stack-based bump allocator for ultra-fast allocations
pub struct BumpAllocator {
    buffer: *mut u8,
    size: usize,
    offset: RefCell<usize>,
}

impl BumpAllocator {
    /// Create a new bump allocator with a fixed buffer size
    pub fn new(size: usize) -> ContractResult<Self> {
        unsafe {
            let layout = Layout::from_size_align(size, 8)
                .map_err(|_| ContractError::InvalidArgument("Invalid layout".into()))?;
            let buffer = alloc(layout);
            if buffer.is_null() {
                return Err(ContractError::StorageWriteFailed);
            }

            Ok(Self {
                buffer,
                size,
                offset: RefCell::new(0),
            })
        }
    }

    /// Allocate memory from the bump allocator
    #[inline(always)]
    pub fn allocate(&self, layout: Layout) -> ContractResult<*mut u8> {
        let mut offset = self.offset.borrow_mut();
        let start = *offset;
        let aligned_start = align_up(start, layout.align());
        let end = aligned_start + layout.size();

        if end > self.size {
            return Err(ContractError::StorageWriteFailed); // Out of memory
        }

        *offset = end;
    unsafe { Ok(self.buffer.add(aligned_start)) }
    }

    /// Reset the allocator
    #[inline(always)]
    pub fn reset(&self) {
        *self.offset.borrow_mut() = 0;
    }

    /// Get current usage
    pub fn used(&self) -> usize {
        *self.offset.borrow()
    }

    /// Get total capacity
    pub fn capacity(&self) -> usize {
        self.size
    }
}

impl Drop for BumpAllocator {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.buffer, Layout::from_size_align(self.size, 8).unwrap());
        }
    }
}

/// Align a value up to the given alignment
#[inline(always)]
const fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}

/// Object pool for frequently allocated types
pub struct ObjectPool<T, const CAPACITY: usize> {
    objects: RefCell<Vec<T>>,
    factory: fn() -> T,
}

impl<T, const CAPACITY: usize> ObjectPool<T, CAPACITY> {
    /// Create a new object pool
    pub fn new(factory: fn() -> T) -> Self {
        Self {
            objects: RefCell::new(Vec::with_capacity(CAPACITY)),
            factory,
        }
    }

    /// Get an object from the pool
    pub fn get(&self) -> T {
        let mut objects = self.objects.borrow_mut();
        objects.pop().unwrap_or_else(|| (self.factory)())
    }

    /// Return an object to the pool
    pub fn put(&self, object: T) {
        let mut objects = self.objects.borrow_mut();
        if objects.len() < CAPACITY {
            objects.push(object);
        }
        // If pool is full, object is dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn test_memory_pool_allocation() {
        let pool = MemoryPool::new();
        let layout = Layout::from_size_align(32, 8).unwrap();

        let ptr = pool.allocate(layout).unwrap();
        assert!(!ptr.is_null());

        pool.deallocate(ptr, layout);
    }

    #[wasm_bindgen_test]
    fn test_bump_allocator() {
        let allocator = BumpAllocator::new(1024).unwrap();
        let layout = Layout::from_size_align(64, 8).unwrap();

        let ptr1 = allocator.allocate(layout).unwrap();
        let ptr2 = allocator.allocate(layout).unwrap();

        assert!(!ptr1.is_null());
        assert!(!ptr2.is_null());
        assert_ne!(ptr1, ptr2);

        allocator.reset();
        assert_eq!(allocator.used(), 0);
    }

    #[wasm_bindgen_test]
    fn test_object_pool() {
        let pool = ObjectPool::new(|| String::from("default"));

        let s1 = pool.get();
        assert_eq!(s1, "default");

        pool.put(s1);
        let s2 = pool.get();
        assert_eq!(s2, "default");
    }
}