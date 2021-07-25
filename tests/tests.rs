use fehler::throws;
use ntest::timeout;

use taskqueue::{Error, QueueType, RunTask, TaskControlCommand, TaskQueue, TaskReceiver, WorkerId};

struct TimeoutTask {
    timeout_sec: u64,
}

impl TimeoutTask {
    fn new(timeout_sec: u64) -> Self {
        Self { timeout_sec }
    }
}

impl RunTask for TimeoutTask {
    fn run(self: Box<Self>, _id: WorkerId, _task_receiver: TaskReceiver) -> TaskControlCommand {
        println!("[D] TimeoutTask started, timeout: {}", self.timeout_sec);
        std::thread::sleep(std::time::Duration::from_secs(self.timeout_sec));
        println!("[D] TimeoutTask finished, timeout: {}", self.timeout_sec);
        TaskControlCommand::Continue
    }
}

#[test]
#[throws]
fn test_task_queue() {
    let task_queue = TaskQueue::new(2, QueueType::Stack);
    task_queue.add_task(Box::new(TimeoutTask::new(2)))?;
    task_queue.add_task(Box::new(TimeoutTask::new(5)))?;
}

struct PanicTimeoutTask {
    timeout_sec: u64,
}

impl PanicTimeoutTask {
    fn new(timeout_sec: u64) -> Self {
        Self { timeout_sec }
    }
}

impl RunTask for PanicTimeoutTask {
    fn run(self: Box<Self>, _id: WorkerId, _task_receiver: TaskReceiver) -> TaskControlCommand {
        println!(
            "[D] PanicTimeoutTask started, timeout: {}",
            self.timeout_sec
        );
        std::thread::sleep(std::time::Duration::from_secs(self.timeout_sec));
        println!(
            "[D] PanicTimeoutTask, finished timeout: {}",
            self.timeout_sec
        );

        panic!("Panic task!");
    }
}

#[test]
#[should_panic]
fn test_task_queue_worker_panic() {
    let task_queue = TaskQueue::new(2, QueueType::Stack);
    task_queue.add_task(Box::new(TimeoutTask::new(2))).ok();
    task_queue.add_task(Box::new(TimeoutTask::new(5))).ok();
    task_queue.add_task(Box::new(PanicTimeoutTask::new(1))).ok();
    task_queue.join().ok();
}

struct RecursiveTimeoutTask {
    timeout_sec: u64,
    deep: u8,
}

impl RecursiveTimeoutTask {
    fn new(timeout_sec: u64, deep: u8) -> Self {
        Self { timeout_sec, deep }
    }
}

impl RunTask for RecursiveTimeoutTask {
    fn run(self: Box<Self>, _id: WorkerId, task_receiver: TaskReceiver) -> TaskControlCommand {
        println!(
            "[D] RecursiveTimeoutTask started, timeout: {}, deep: {}",
            self.timeout_sec, self.deep
        );
        std::thread::sleep(std::time::Duration::from_secs(self.timeout_sec));
        println!(
            "[D] RecursiveTimeoutTask finished, timeout: {}, deep: {}",
            self.timeout_sec, self.deep
        );

        if self.deep > 0 {
            task_receiver.add_task(Box::new(RecursiveTimeoutTask::new(
                self.timeout_sec,
                self.deep - 1,
            )));
        }

        TaskControlCommand::Continue
    }
}

#[test]
#[throws]
fn test_task_queue_recursive() {
    let task_queue = TaskQueue::new(2, QueueType::Stack);
    task_queue.add_task(Box::new(RecursiveTimeoutTask::new(2, 4)))?;
    task_queue.add_task(Box::new(RecursiveTimeoutTask::new(5, 2)))?;
    task_queue.join()?;
}

#[test]
#[should_panic]
fn test_task_queue_recursive_with_panic() {
    let task_queue = TaskQueue::new(2, QueueType::Queue);
    task_queue
        .add_task(Box::new(RecursiveTimeoutTask::new(2, 4)))
        .ok();
    task_queue
        .add_task(Box::new(RecursiveTimeoutTask::new(5, 2)))
        .ok();
    task_queue.add_task(Box::new(PanicTimeoutTask::new(4))).ok();
    task_queue.join().ok();
}

#[test]
#[timeout(10000)]
#[throws]
fn test_task_queue_abort() {
    let task_queue = TaskQueue::new(2, QueueType::Stack);
    task_queue
        .add_task(Box::new(RecursiveTimeoutTask::new(4, 200)))
        .ok();
    task_queue
        .add_task(Box::new(RecursiveTimeoutTask::new(2, 200)))
        .ok();
    task_queue.abort().ok();
}

struct AbortTask {
    timeout_sec: u64,
}

impl AbortTask {
    fn new(timeout_sec: u64) -> Self {
        Self { timeout_sec }
    }
}

impl RunTask for AbortTask {
    fn run(self: Box<Self>, _id: WorkerId, _task_receiver: TaskReceiver) -> TaskControlCommand {
        println!("[D] AbortTask started, timeout: {}", self.timeout_sec);
        std::thread::sleep(std::time::Duration::from_secs(self.timeout_sec));
        println!("[D] AbortTask finished, timeout: {}", self.timeout_sec);
        TaskControlCommand::Abort
    }
}

#[test]
#[timeout(10000)]
#[throws]
fn test_task_queue_inner_abort() {
    let task_queue = TaskQueue::new(2, QueueType::Stack);
    task_queue
        .add_task(Box::new(RecursiveTimeoutTask::new(4, 200)))
        .ok();
    task_queue.add_task(Box::new(AbortTask::new(2))).ok();
    task_queue.abort().ok();
}

#[cfg(feature = "fn_traits")]
#[test]
#[throws]
fn test_closure_as_task() {
    let task_queue = TaskQueue::new(2, QueueType::Stack);
    task_queue.add_task(Box::new(|_id, _task_receiver| {
        println!("[D] Closure task called.");
        TaskControlCommand::Continue
    }));
}
