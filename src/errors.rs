use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Empty worker thread handle"))]
    EmptyWorkerThreadHandle,

    #[snafu(display("Empty controller thread handle"))]
    EmptyControllerThreadHandle,

    #[snafu(display("Worker not found, id: {}", id))]
    WorkerNotFound { id: usize },

    #[snafu(display("Sync poison error: {}", description))]
    PoisonError { description: String },

    #[snafu(display("Task queue send task error. {}", explanation))]
    TaskQueueSendError { explanation: String },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
