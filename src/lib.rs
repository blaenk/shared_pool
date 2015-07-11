extern crate libc;
extern crate eventual;
extern crate syncbox;
extern crate num_cpus;

use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};
use std::mem;
use std::fmt;
use std::error;

use syncbox::{ThreadPool, TaskBox};

mod macros;

static POOL: AtomicUsize = ATOMIC_USIZE_INIT;

const UNINITIALIZED: usize = 0;
const INITIALIZING: usize = 1;

pub fn init(size: usize) -> Result<(), InitPoolError> {
    if POOL.compare_and_swap(UNINITIALIZED, INITIALIZING, Ordering::SeqCst)
       != UNINITIALIZED {
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

/// The type returned by `init` if `init` has already been called.
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

#[inline]
fn to_pool(ptr: usize) -> &'static ThreadPool<Box<TaskBox>> {
    unsafe { mem::transmute(ptr) }
}

pub fn pool() -> Result<ThreadPool<Box<TaskBox>>, InitPoolError> {
    let ptr = POOL.load(Ordering::SeqCst);

    // I think this is OK because initializing right here
    // would atomically set it to INITIALIZING
    // thereby preventing races during initialization
    if ptr == UNINITIALIZED || ptr == INITIALIZING {
        try!(init(num_cpus::get()));

        let ptr = POOL.load(Ordering::SeqCst);

        assert!(ptr != UNINITIALIZED && ptr != INITIALIZING);

        Ok(to_pool(ptr).clone())
    } else {
        Ok(to_pool(ptr).clone())
    }
}

#[test]
fn test_pool() {
    use syncbox::Run;

    init(4).unwrap();

    let pool: ThreadPool<Box<TaskBox>> = pool().unwrap();
}
