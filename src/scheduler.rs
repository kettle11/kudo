// This implementation probably contains too many `Arc`s.
// Is there a way to reduce it? 

use ktasks::*;
use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};

struct Task {
    waiting_on: AtomicUsize,
    will_wait_on: usize,
    task: Arc<Box<dyn Fn() + Send + Sync>>,
    tasks_to_wake_up: Arc<Vec<usize>>,
    // I'm not fond of this inner RwLock.
    // Is there a way to not use it it?
    all_tasks: Arc<RwLock<Vec<Task>>>,
}

impl Task {
    pub fn new(
        will_wait_on: usize,
        tasks_to_wake_up: Vec<usize>,
        all_tasks: Arc<RwLock<Vec<Task>>>,
        task: Box<dyn Fn() + Send + Sync>,
    ) -> Self {
        Self {
            waiting_on: AtomicUsize::new(will_wait_on),
            tasks_to_wake_up: Arc::new(tasks_to_wake_up),
            will_wait_on,
            task: Arc::new(task),
            all_tasks,
        }
    }

    pub fn decrement_and_try_run(&self) {
        self.waiting_on.fetch_sub(1, Ordering::Relaxed);

        // Thought is needed here about the ordering.
        while self.waiting_on.load(Ordering::Relaxed) <= 0 {
            // I have no idea if this is the correct ordering.
            self.waiting_on
                .fetch_add(self.will_wait_on, Ordering::Relaxed);
            // It should be scheduled here instead.
            let task = self.task.clone();

            let all_tasks = self.all_tasks.clone();
            let tasks_to_wake_up = self.tasks_to_wake_up.clone();

            spawn(async move {
                task();
                let all_tasks = all_tasks.read().unwrap();
                for task in tasks_to_wake_up.iter() {
                    all_tasks[*task].decrement_and_try_run();
                }
            })
            .run();
        }
    }
}

#[derive(Debug, Clone)]
// Indices to Task
enum NextUp {
    None,
    // The previous write task blocked on
    Readers((Option<usize>, Vec<usize>)),
    Writer(usize),
}

struct TaskInfo {
    waiting_for: usize,
    tasks_to_wake_up: Vec<usize>,
    task: Box<dyn Fn() + Send + Sync>,
}

struct Scheduler {
    task_info: Vec<TaskInfo>,
    resources: Vec<NextUp>,
    /// Tasks that will run at the start of schedule cycle.
    starter_tasks: Vec<usize>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            task_info: Vec::new(),
            resources: Vec::new(),
            starter_tasks: Vec::new(),
        }
    }

    // Probably those shouldn't be allowed.
    pub fn add_task(
        &mut self,
        reads: &[usize],
        writes: &[usize],
        task: Box<dyn Fn() + Send + Sync>,
    ) {
        // Only tasks with reads or writes
        assert!(reads.len() + writes.len() > 0);

        // Todo: Add a debug check that reads and writes don't overlap.

        let task_id = self.task_info.len();
        let mut waiting_for = 0;

        // Used to ensure that each task only wakes up the next task once if they share multiple resources.
        let mut wakers = HashSet::new();

        for read in reads.iter() {
            // Ensure that a resource slot is allocated for each resource ID we're scheduling with.
            self.resources
                .resize(self.resources.len().max(*read + 1), NextUp::None);

            match &mut self.resources[*read] {
                NextUp::Readers((blocked_on, readers)) => {
                    if let Some(blocked_on) = blocked_on {
                        if wakers.insert(*blocked_on) {
                            waiting_for += 1;
                            self.task_info[*blocked_on].tasks_to_wake_up.push(task_id);
                        }
                    }
                    readers.push(task_id)
                }
                NextUp::Writer(writer) => {
                    if wakers.insert(*writer) {
                        waiting_for += 1;
                        self.task_info[*writer].tasks_to_wake_up.push(task_id);
                    }
                    self.resources[*read] = NextUp::Readers((Some(*writer), vec![task_id]));
                }
                NextUp::None => {
                    self.resources[*read] = NextUp::Readers((None, vec![task_id]));
                }
            }
        }

        for write in writes.iter() {
            // Ensure that a resource slot is allocated for each resource ID we're scheduling with.
            self.resources
                .resize(self.resources.len().max(*write + 1), NextUp::None);
            match &mut self.resources[*write] {
                NextUp::Readers((_, readers)) => {
                    for reader in readers.iter() {
                        if wakers.insert(*reader) {
                            waiting_for += 1;
                            self.task_info[*reader].tasks_to_wake_up.push(task_id);
                        }
                    }
                    self.resources[*write] = NextUp::Writer(task_id);
                }
                NextUp::Writer(writer) => {
                    if wakers.insert(*writer) {
                        waiting_for += 1;
                        self.task_info[*writer].tasks_to_wake_up.push(task_id);
                    }
                    self.resources[*write] = NextUp::Writer(task_id);
                }
                NextUp::None => {
                    self.resources[*write] = NextUp::Writer(task_id);
                }
            }
        }

        if waiting_for == 0 {
            self.starter_tasks.push(task_id);
        }

        self.task_info.push(TaskInfo {
            waiting_for,
            tasks_to_wake_up: Vec::new(),
            task,
        })
    }

    pub fn run(mut self) {
        for task in self.starter_tasks.iter() {
            self.task_info[*task].waiting_for += 1;
        }

        // It's tricky to construct these interdependent things successfully.
        // The approach used here is a bit messy and inefficient.
        // But it should be OK for now for non-massive amounts of tasks.

        let mut new_tasks = Vec::with_capacity(self.task_info.len());
        let all_tasks = Arc::new(RwLock::new(Vec::new()));

        for task in self.task_info {
            new_tasks.push(Task::new(
                task.waiting_for,
                task.tasks_to_wake_up,
                all_tasks.clone(),
                task.task,
            ))
        }

        *all_tasks.write().unwrap() = new_tasks;

        // Create the tasks and then enqueue them.

        let all_tasks = all_tasks.read().unwrap();
        for task in self.starter_tasks {
            println!("STARTING TASK: {:?}", task);
            all_tasks[task].decrement_and_try_run();
        }
    }
}

// Tasks must be declared in order.

fn main() {
    create_workers(3);

    let mut scheduler = Scheduler::new();
    scheduler.add_task(&[0], &[], Box::new(|| println!("TASK 0")));
    scheduler.add_task(
        &[0],
        &[1],
        Box::new(|| {
            std::thread::sleep(std::time::Duration::from_millis(5));
            println!("TASK 1")
        }),
    );
    scheduler.add_task(&[], &[1], Box::new(|| println!("TASK 2")));
    scheduler.add_task(&[0], &[], Box::new(|| println!("TASK 3")));

    for (i, task) in scheduler.task_info.iter().enumerate() {
        println!("TASK {:?}", i);
        println!("  WAITING FOR: {:?}", task.waiting_for);
        println!("  WILL WAKE UP: {:?}", task.tasks_to_wake_up);
    }

    scheduler.run();
    run_current_thread_tasks();
    std::thread::sleep(std::time::Duration::from_millis(20));

    // println!("{:#?}", scheduler.task_info);
}
