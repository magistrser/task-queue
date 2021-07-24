use fehler::throws;
use std::collections::HashMap;

use crate::{errors, ControllerToWorkerChanelSender, Error, Panic};

pub type WorkerId = usize;
pub type Workers = HashMap<WorkerId, Worker>;

pub struct Worker {
    thread: Option<std::thread::JoinHandle<()>>,
    pub channel: ControllerToWorkerChanelSender,
}

impl Drop for Worker {
    fn drop(&mut self) {
        if self.thread.is_some() {
            panic!("WorkerHandle::drop, 'join' not called on worker thread");
        }
    }
}

impl Worker {
    pub(crate) fn new(
        thread: std::thread::JoinHandle<()>,
        channel: ControllerToWorkerChanelSender,
    ) -> Self {
        Self {
            thread: Some(thread),
            channel,
        }
    }

    #[throws]
    pub fn join(&mut self) -> Panic {
        self.thread
            .take()
            .ok_or_else(|| errors::EmptyWorkerThreadHandle.build())?
            .join()
            .err()
    }
}
