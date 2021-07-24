pub mod errors;
pub use errors::{Error, Result};
mod events;
use events::*;
mod workers_holder;
use workers_holder::*;
pub mod task;
pub use task::*;
mod worker;
pub use worker::WorkerId;
use worker::*;

use fehler::{throw, throws};
use std::{
    collections::LinkedList,
    sync::{mpsc, Arc, Mutex},
};

macro_rules! break_loop_if_error {
    ($l: expr) => {
        match $l {
            Ok(result) => result,
            Err(_) => break,
        }
    };
}

macro_rules! lock_arc_mutex {
    ($l: expr) => {
        $l.lock().map_err(|error| {
            errors::PoisonError {
                description: error.to_string(),
            }
            .build()
        })
    };
}

type ControllerToWorkerChanelSender = mpsc::Sender<ControlToWorkerEvent>;
type ControllerToWorkerChanelReceiver = mpsc::Receiver<ControlToWorkerEvent>;
type WorkerToControllerChanelSender = mpsc::Sender<WorkerToControlEvent>;
type WorkerToControllerChanelReceiver = mpsc::Receiver<WorkerToControlEvent>;

type ArcMutex<T> = Arc<Mutex<T>>;
type Panic = Option<Box<dyn std::any::Any + Send + 'static>>;

#[derive(Copy, Clone)]
pub enum QueueType {
    Stack,
    Queue,
}

pub type TaskContainer = LinkedList<Task>;

pub struct TaskQueue {
    controller_handle: Option<std::thread::JoinHandle<errors::Result<Panic>>>,
    worker_to_controller_channel_sender: WorkerToControllerChanelSender,
}

impl Drop for TaskQueue {
    fn drop(&mut self) {
        self.join_inner(|_| {
            // ignore error, for error handling use join
            Ok(())
        })
        .ok();
    }
}

impl TaskQueue {
    fn run_worker(
        id: WorkerId,
        task_queue: ArcMutex<TaskContainer>,
        from_controller_channel: ControllerToWorkerChanelReceiver,
        to_controller_channel: WorkerToControllerChanelSender,
    ) {
        loop {
            match break_loop_if_error!(from_controller_channel.recv()) {
                ControlToWorkerEvent::Stop => break,
                ControlToWorkerEvent::KeepAlive => continue,
                ControlToWorkerEvent::RunTask(task) => {
                    let task_receiver = TaskReceiver::new(to_controller_channel.clone());
                    match task.run(id, task_receiver) {
                        TaskControlCommand::Abort => break_loop_if_error!(to_controller_channel
                            .clone()
                            .send(WorkerToControlEvent::AbortTaskChain)),
                        TaskControlCommand::Continue => {}
                    }

                    loop {
                        let task = { break_loop_if_error!(task_queue.lock()).pop_back() };
                        if let Some(task) = task {
                            let task_receiver = TaskReceiver::new(to_controller_channel.clone());
                            match task.run(id, task_receiver) {
                                TaskControlCommand::Abort => {
                                    break_loop_if_error!(to_controller_channel
                                        .clone()
                                        .send(WorkerToControlEvent::AbortTaskChain))
                                }
                                TaskControlCommand::Continue => continue,
                            }
                        }
                        break;
                    }

                    break_loop_if_error!(to_controller_channel
                        .clone()
                        .send(WorkerToControlEvent::Complete(id)));
                }
            }
        }
    }

    #[throws]
    fn run_controller(
        workers: Workers,
        queue_type: QueueType,
        task_queue: ArcMutex<TaskContainer>,
        worker_to_controller_channel_receiver: WorkerToControllerChanelReceiver,
    ) -> Panic {
        let mut controller = WorkersHolder::new(workers);

        let mut stop_flag = false;

        loop {
            match worker_to_controller_channel_receiver
                .recv_timeout(std::time::Duration::from_secs(10))
            {
                Ok(event) => match event {
                    WorkerToControlEvent::AddTask(task) => {
                        Self::add_to_queue(task, queue_type, &task_queue)?
                    }
                    WorkerToControlEvent::Complete(id) => controller.free_worker(id)?,
                    WorkerToControlEvent::AbortTaskChain => {
                        let mut queue = lock_arc_mutex!(task_queue)?;
                        queue.clear();
                        break;
                    }
                    // ControlEvents
                    WorkerToControlEvent::Stop => stop_flag = true,
                },
                _ => {}
            }

            if controller.keep_alive()? {
                break;
            }

            {
                let mut queue = lock_arc_mutex!(task_queue)?;
                if stop_flag && !controller.has_busy_workers() && queue.is_empty() {
                    break;
                }
                controller.run_tasks(&mut *queue)?;
            }
        }

        controller.stop_workers()?;
        controller.join()?
    }

    #[throws]
    fn add_to_queue(task: Task, queue_type: QueueType, queue: &ArcMutex<TaskContainer>) {
        match queue_type {
            QueueType::Stack => lock_arc_mutex!(queue)?.push_back(task),
            QueueType::Queue => lock_arc_mutex!(queue)?.push_front(task),
        }
    }

    pub fn new(workers_count: usize, queue_type: QueueType) -> Self {
        let (worker_to_controller_channel_sender, worker_to_controller_channel_receiver) =
            mpsc::channel();
        let task_queue = Arc::new(Mutex::new(TaskContainer::new()));
        let mut workers = Workers::new();

        for id in 0..workers_count {
            let task_queue_copy = task_queue.clone();
            let worker_to_controller_channel_sender = worker_to_controller_channel_sender.clone();
            let (controller_to_worker_channel_sender, controller_to_worker_channel_receiver) =
                mpsc::channel();

            let worker_handle = Worker::new(
                std::thread::spawn(move || {
                    Self::run_worker(
                        id,
                        task_queue_copy,
                        controller_to_worker_channel_receiver,
                        worker_to_controller_channel_sender,
                    )
                }),
                controller_to_worker_channel_sender.clone(),
            );

            workers.insert(id, worker_handle);
        }

        let task_queue_copy = task_queue.clone();
        let controller_handle = Some(std::thread::spawn(move || {
            Self::run_controller(
                workers,
                queue_type,
                task_queue_copy,
                worker_to_controller_channel_receiver,
            )
        }));

        Self {
            controller_handle,
            worker_to_controller_channel_sender,
        }
    }

    #[throws]
    pub fn add_task(&self, task: Task) {
        self.worker_to_controller_channel_sender
            .send(WorkerToControlEvent::AddTask(task))
            .map_err(|error| {
                errors::TaskQueueSendError {
                    explanation: error.to_string(),
                }
                .build()
            })?;
    }

    #[throws]
    pub fn abort(mut self) {
        self.worker_to_controller_channel_sender
            .send(WorkerToControlEvent::AbortTaskChain)
            .map_err(|error| {
                errors::TaskQueueSendError {
                    explanation: error.to_string(),
                }
                .build()
            })?;
        self.join_inner(|control_error| throw!(control_error))?;
    }

    #[throws]
    pub fn join(mut self) {
        self.join_inner(|control_error| throw!(control_error))?;
    }

    #[throws]
    fn join_inner(&mut self, error_handle: impl FnOnce(errors::Error) -> errors::Result<()>) {
        self.worker_to_controller_channel_sender
            .send(WorkerToControlEvent::Stop)
            .ok();

        match self
            .controller_handle
            .take()
            .ok_or_else(|| errors::EmptyControllerThreadHandle.build())?
            .join()
        {
            Ok(controller_result) => match controller_result {
                Ok(result) => {
                    if let Some(worker_panic) = result {
                        std::panic::resume_unwind(worker_panic);
                    }
                }
                Err(control_error) => error_handle(control_error)?,
            },
            Err(controller_panic) => std::panic::resume_unwind(controller_panic),
        }
    }
}
