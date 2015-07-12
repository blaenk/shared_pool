extern crate libc;
extern crate eventual;
extern crate syncbox;
extern crate num_cpus;

use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};
use std::mem;
use std::fmt;
use std::error;

use syncbox::{ThreadPool, TaskBox};

static POOL: AtomicUsize = ATOMIC_USIZE_INIT;

const UNINITIALIZED: usize = 0;
const INITIALIZING: usize = 1;

macro_rules! pool {
    () => {
        pool!(POOL)
    };

    ($name:ident) => {
        $crate::pool_from(&::$name)
    }
}

macro_rules! init_pool {
    ($size:expr) => {
        init_pool!(POOL, $size)
    };

    ($name:ident, $size:expr) => {{
        extern fn $name() {
            // Set to INITIALIZING to prevent re-initialization after
            let pool = ::$name.swap($crate::INITIALIZING, Ordering::SeqCst);

            unsafe {
                // trigger drop
                mem::transmute::<usize, Box<ThreadPool<Box<TaskBox>>>>(pool);
            }
        }

        $crate::init_from(&::$name, $size).and_then(|_| {
            unsafe {
                assert_eq!(libc::atexit($name), 0);
            }

            Ok(())
        })
    }}
}

// pub fn init(size: usize) -> Result<(), InitPoolError> {
//     init_pool!(POOL, size)
// }

pub fn init_from(handle: &'static AtomicUsize, size: usize) -> Result<(), InitPoolError> {
    if handle.compare_and_swap(UNINITIALIZED, INITIALIZING, Ordering::SeqCst)
       != UNINITIALIZED {
        return Err(InitPoolError(()));
    }

    let pool = Box::new(ThreadPool::fixed_size(size as u32));
    let pool = unsafe {
        mem::transmute::<Box<ThreadPool<Box<TaskBox>>>, usize>(pool)
    };

    handle.store(pool, Ordering::SeqCst);

    return Ok(());
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

// pub fn pool() -> Result<ThreadPool<Box<TaskBox>>, InitPoolError> {
//     pool!(POOL)
// }

pub fn pool_from(handle: &'static AtomicUsize)
    -> Result<ThreadPool<Box<TaskBox>>, InitPoolError> {
    let ptr = handle.load(Ordering::SeqCst);

    // I think this is OK because initializing right here
    // would atomically set it to INITIALIZING
    // thereby preventing races during initialization
    if ptr == UNINITIALIZED || ptr == INITIALIZING {
        try!(init_from(handle, num_cpus::get()));

        let ptr = handle.load(Ordering::SeqCst);

        assert!(ptr != UNINITIALIZED && ptr != INITIALIZING);

        Ok(to_pool(ptr).clone())
    } else {
        Ok(to_pool(ptr).clone())
    }
}

static CUSTOM_POOL: AtomicUsize = ATOMIC_USIZE_INIT;

#[test]
fn test_default_pool() {
    use syncbox::{Run, TaskBox, ThreadPool};

    init_pool!(4).unwrap();

    let pool: ThreadPool<Box<TaskBox>> = pool!().unwrap();

    pool.run(Box::new(move || {
        println!("in pool");
    }));

    init_pool!(CUSTOM_POOL, 4).unwrap();

    let pool: ThreadPool<Box<TaskBox>> = pool!(CUSTOM_POOL).unwrap();

    pool.run(Box::new(move || {
        println!("in pool");
    }));
}
