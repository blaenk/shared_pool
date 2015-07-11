extern crate libc;
extern crate syncbox;

use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};
use std::mem;
use std::fmt;
use std::error;

use syncbox::{ThreadPool, TaskBox};

static POOL: AtomicUsize = ATOMIC_USIZE_INIT;

const UNINITIALIZED: usize = 0;
const INITIALIZING: usize = 1;

/// Initialize the shared pool.
///
/// Returns an error if the pool has already been initialized

pub fn init(size: usize) -> Result<(), InitPoolError> {
    let val = POOL.compare_and_swap(UNINITIALIZED, INITIALIZING, Ordering::SeqCst);

    if val != UNINITIALIZED {
        return Err(InitPoolError(()));
    }

    let pool = Box::new(ThreadPool::fixed_size(size as u32));

    let pool = unsafe {
        mem::transmute::<Box<ThreadPool<Box<TaskBox>>>, usize>(pool)
    };

    POOL.store(pool, Ordering::SeqCst);

    unsafe {
        assert_eq!(libc::atexit(shutdown), 0);
    }

    return Ok(());

    extern fn shutdown() {
        // Set to INITIALIZING to prevent re-initialization after
        let logger = POOL.swap(INITIALIZING, Ordering::SeqCst);

        unsafe {
            // trigger drop
            mem::transmute::<usize, Box<ThreadPool<Box<TaskBox>>>>(logger);
        }
    }
}

/// An error representing an attempt to initialize the pool more than once.

#[allow(missing_copy_implementations)]
#[derive(Debug)]
pub struct InitPoolError(());

impl fmt::Display for InitPoolError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "attempted to initialize a pool after the pool \
                     was already initialized")
    }
}

impl error::Error for InitPoolError {
    fn description(&self) -> &str { "init() called multiple times" }
}

/// Retrieve an instance of the shared pool if it has been initialized.

pub fn get() -> Option<ThreadPool<Box<TaskBox>>> {
    let ptr = POOL.load(Ordering::SeqCst);

    if ptr == UNINITIALIZED || ptr == INITIALIZING {
        None
    } else {
        let pool: &'static ThreadPool<Box<TaskBox>> = unsafe {
            mem::transmute(ptr)
        };

        Some(pool.clone())
    }
}

/// Retrieve an instance of the shared pool if it exists, otherwise
/// initialize one and retrieve an instance to it.

pub fn get_or_init(size: usize) -> Result<ThreadPool<Box<TaskBox>>, InitPoolError> {
    let ptr = POOL.load(Ordering::SeqCst);

    let ptr = if ptr == UNINITIALIZED || ptr == INITIALIZING {
        try!(init(size));
        ptr
    } else {
        ptr
    };

    let pool: &'static ThreadPool<Box<TaskBox>> = unsafe {
        mem::transmute(ptr)
    };

    Ok(pool.clone())
}

#[test]
fn test_pool() {
    init(4).unwrap();

    let _pool: ThreadPool<Box<TaskBox>> = get().unwrap();
}
