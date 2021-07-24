use crate::{events::WorkerToControlEvent, worker::WorkerId, WorkerToControllerChanelSender};

pub struct TaskReceiver {
    channel: WorkerToControllerChanelSender,
}

impl TaskReceiver {
    pub(crate) fn new(channel: WorkerToControllerChanelSender) -> Self {
        Self { channel }
    }

    pub fn add_task(&self, task: Task) {
        self.channel.send(WorkerToControlEvent::AddTask(task)).ok();
    }
}

pub enum TaskControlCommand {
    Continue,
    Abort,
}

pub trait RunTask {
    fn run(self: Box<Self>, id: WorkerId, task_receiver: TaskReceiver) -> TaskControlCommand;
}

pub type Task = Box<dyn RunTask + Send>;
