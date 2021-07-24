use crate::{task::Task, worker::WorkerId};

pub enum ControlToWorkerEvent {
    Stop,
    KeepAlive,
    RunTask(Task),
}

pub enum WorkerToControlEvent {
    Complete(WorkerId),
    AddTask(Task),
    AbortTaskChain,
    // ControlEvents
    Stop,
}
