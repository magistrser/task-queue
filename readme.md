## Task queue
---
### Description
Parallel execution of the task queue with the ability to add new tasks inside the running tasks

---
## Examples
```rust
let thread_count = 2;
let queue_type = QueueType::Stack;
let task_queue = TaskQueue::new(thread_count, queue_type);
```
where
`thread_count` - the number of threads that execute tasks in parallel
`queue_type` -
determines at the beginning or at the end of the queue the task will be added (Available values: Queue, Stack)

```rust
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
        std::thread::sleep(std::time::Duration::from_secs(self.timeout_sec));

        if self.deep > 0 {
            task_receiver.add_task(Box::new(RecursiveTimeoutTask::new(
                self.timeout_sec,
                self.deep - 1,
            )));
        }

        TaskControlCommand::Continue
    }
}
```

In order to add a task to the queue, you need to implement the trait `RunTask`. The third argument `task_receiver` is used to add new tasks. Method `run` returns `TaskControlCommand`, availabale values `Continue` - default value that does not affect the operation of the queue in any way, `Abort` - reset outstanding tasks and do not take new ones, the task queue will no longer execute tasks added externally.

```rust
// Add new task
task_queue.add_task(Box::new(RecursiveTimeoutTask::new(2, 4)))?;
task_queue.add_task(Box::new(RecursiveTimeoutTask::new(5, 2)))?;

// Cancel tasks and wait for the completion of task processing (Analogue - TaskControlCommand::Abort)
task_queue.abort()?;

// Wait untill all tasks are completed
task_queue.join()?;
```

! If you do not use `abort` /` join`, drop will be used, but panics from worker processes are not handled, use join instead.