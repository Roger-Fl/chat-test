use std::fs::File;
use std::ops::DerefMut;
use std::path::Path;
use std::io::Result;
use std::sync::Mutex;
use std::io::Write;

pub struct Logger {
    // file: Mutex<File>,
    writer: Mutex<Option<Box<dyn Write>>>,
}

impl Logger {
    pub fn new(writer: Option<Box<dyn Write>>) -> Self {
        Logger {
            writer: Mutex::new(writer),
        }
    }

    // pub fn apply(&self, f: impl FnOnce(&mut Box<dyn Write>)) {
    //     let mut inner = self.writer.lock().expect("Failed to lock mutex");
    //     f(&mut *inner);
    // }
    // pub fn create<P: AsRef<Path>>(path: P) -> Result<Logger> {
    //     let file = File::create(path);
    //     let file = match file {
    //         Ok(file) => file,
    //         Err(e) => return Err(e),
    //     };

    //     let logger = Logger {
    //         file: Mutex::new(file)
    //     };
    //     Ok(logger)
    // }

    pub fn log(&self, buffer: &[u8]) -> Result<usize> {
        let mut lock_guard = self.writer.lock().expect("Failed to lock mutex");
        if lock_guard.is_some() {
            let log = lock_guard.as_mut().unwrap();
            let res = [log.write(buffer), log.write("\n".as_bytes())];

            return res.into_iter().sum();
        }

        Ok(0)
    }
    pub fn is_set(&self) -> bool {
        self.writer.lock().expect("Failed to lock mutex").is_some()
    }
}

unsafe impl Sync for Logger{}
unsafe impl Send for Logger{}