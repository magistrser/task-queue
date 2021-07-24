use fehler::throws;
use std::collections::HashMap;

use crate::{
    errors,
    events::ControlToWorkerEvent,
    worker::{WorkerId, Workers},
    Error, Panic, TaskContainer,
};

enum Status {
    Free,
    Busy,
    Released,
}
type WorkersStatus = HashMap<WorkerId, Status>;

pub struct WorkersHolder {
    workers: Workers,
    workers_status: WorkersStatus,
}

impl WorkersHolder {
    pub fn new(workers: Workers) -> Self {
        let mut workers_status = WorkersStatus::new();
        for (id, _worker) in workers.iter() {
            workers_status.insert(*id, Status::Free);
        }

        Self {
            workers,
            workers_status,
        }
    }

    #[throws]
    pub fn free_worker(&mut self, id: WorkerId) {
        *(self
            .workers_status
            .get_mut(&id)
            .ok_or_else(|| errors::WorkerNotFound { id }.build())?) = Status::Free;
    }

    pub fn has_busy_workers(&self) -> bool {
        self.workers_status
            .iter()
            .any(|(_, status)| matches!(status, Status::Busy))
    }

    #[throws]
    pub fn run_tasks(&mut self, task_queue: &mut TaskContainer) {
        for (id, status) in self.workers_status.iter_mut() {
            if matches!(status, Status::Free) {
                if let Some(task) = task_queue.pop_back() {
                    self.workers
                        .get(id)
                        .ok_or_else(|| errors::WorkerNotFound { id: *id }.build())?
                        .channel
                        .send(ControlToWorkerEvent::RunTask(task))
                        .ok();

                    *status = Status::Busy;
                } else {
                    break;
                }
            }
        }
    }

    #[throws]
    pub fn keep_alive(&mut self) -> bool {
        let mut has_crashed_workers = false;
        for (id, status) in self.workers_status.iter_mut() {
            if matches!(status, Status::Busy) {
                let is_err = self
                    .workers
                    .get(id)
                    .ok_or_else(|| errors::WorkerNotFound { id: *id }.build())?
                    .channel
                    .send(ControlToWorkerEvent::KeepAlive)
                    .is_err();

                if is_err {
                    *status = Status::Released;
                    has_crashed_workers = true;
                }
            }
        }

        has_crashed_workers
    }

    #[throws]
    pub fn stop_workers(&mut self) {
        for (id, status) in self.workers_status.iter_mut() {
            if matches!(status, Status::Busy | Status::Free) {
                self.workers
                    .get(id)
                    .ok_or_else(|| errors::WorkerNotFound { id: *id }.build())?
                    .channel
                    .send(ControlToWorkerEvent::Stop)
                    .ok();

                *status = Status::Released;
            }
        }
    }

    #[throws]
    pub fn join(&mut self) -> Panic {
        let mut last_panic = None;

        for (_id, worker) in self.workers.iter_mut() {
            if let Some(worker_panic) = worker.join()? {
                last_panic = Some(worker_panic);
            }
        }

        last_panic
    }
}
