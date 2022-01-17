use std::time::Instant;

use lazy_static::lazy_static;
use std::sync::{Mutex, MutexGuard};

lazy_static! {
    static ref FPS_COUNTER: Mutex<FpsCounter> = Mutex::new(FpsCounter::default());
}

pub fn default_counter() -> MutexGuard<'static, FpsCounter> {
    FPS_COUNTER.lock().unwrap()
}
unsafe impl Send for FpsCounter {}
unsafe impl Sync for FpsCounter {}

pub struct FpsCounter {
    count: u64,
    time: Instant
}
impl Default for FpsCounter {
    fn default() -> Self {
        Self {
            count: 0,
            time: Instant::now()
        }
    }
}

impl FpsCounter {
    pub fn frame(&mut self) {
        self.count+=1;
    }

    pub fn report(&mut self) {
        if self.count % 100 == 0 {
            log::debug!("FPS: {}", self.count as f64 / self.time.elapsed().as_secs_f64());
        }
        if self.count % (60 * 3) == 0 {
            self.count = 0;
            self.time = Instant::now();
        }
    }
}