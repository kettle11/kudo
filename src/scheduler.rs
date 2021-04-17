// This implementation probably contains too many `Arc`s.
// Is there a way to reduce it?

use ktasks::*;
use std::sync::{Arc, RwLock};
use std::{collections::HashSet, sync::Mutex};
use std::{
    panic::Location,
    sync::atomic::{AtomicUsize, Ordering},
};

struct Task {
    waiting_on: AtomicUsize,
    will_wait_on: usize,
    // This Mutex is probably not necessary because the scheduler
    // *should* prevent contention but for now it's simplest / safest to
    // just use a Mutex here to make task Sync.
    task: Arc<Mutex<Box<dyn FnMut() + Send>>>,
    tasks_to_wake_up: Arc<Vec<usize>>,
    // I'm not fond of this inner RwLock.
    // Is there a way to not use it it?
    all_tasks: Arc<RwLock<Vec<Task>>>,
    main_thread_task: bool,
}

impl Task {
    pub fn new(
        will_wait_on: usize,
        tasks_to_wake_up: Vec<usize>,
        all_tasks: Arc<RwLock<Vec<Task>>>,
        task: Box<dyn FnMut() + Send>,
        main_thread_task: bool,
    ) -> Self {
        Self {
            waiting_on: AtomicUsize::new(will_wait_on),
            tasks_to_wake_up: Arc::new(tasks_to_wake_up),
            will_wait_on,
            task: Arc::new(Mutex::new(task)),
            all_tasks,
            main_thread_task,
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

            let inner_spawn = async move {
                (task.lock().unwrap())();
                let all_tasks = all_tasks.read().unwrap();
                for task in tasks_to_wake_up.iter() {
                    all_tasks[*task].decrement_and_try_run();
                }
            };

            if self.main_thread_task {
                spawn_main(inner_spawn).run();
            } else {
                spawn(inner_spawn).run();
            }
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
    task: Box<dyn FnMut() + Send>,
    main_thread_task: bool,
}

struct Scheduler {
    task_info: Vec<TaskInfo>,
    resources: Vec<NextUp>,
    /// Tasks that will run at the start of schedule cycle.
    starter_tasks: Vec<usize>,
    last_exclusive_task: Option<usize>,
    last_task: Option<usize>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            task_info: Vec::new(),
            resources: Vec::new(),
            starter_tasks: Vec::new(),
            last_exclusive_task: None,
            last_task: None,
        }
    }

    pub fn add_task(
        &mut self,
        reads: impl IntoIterator<Item = usize>,  //&[usize],
        writes: impl IntoIterator<Item = usize>, //&[usize],
        task: Box<dyn FnMut() + Send>,
        exclusive_task: bool,
        main_thread_task: bool,
    ) {
        // Only allow tasks with reads or writes
        // assert!(reads.len() + writes.len() > 0);

        // Todo: Add a debug check that reads and writes don't overlap.

        let task_id = self.task_info.len();
        let mut waiting_for = 0;

        // Used to ensure that each task only wakes up the next task once if they share multiple resources.
        let mut wakers = HashSet::new();

        for read in reads {
            // Ensure that a resource slot is allocated for each resource ID we're scheduling with.
            self.resources
                .resize(self.resources.len().max(read + 1), NextUp::None);

            match &mut self.resources[read] {
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
                    self.resources[read] = NextUp::Readers((Some(*writer), vec![task_id]));
                }
                NextUp::None => {
                    self.resources[read] = NextUp::Readers((None, vec![task_id]));
                }
            }
        }

        for write in writes {
            // Ensure that a resource slot is allocated for each resource ID we're scheduling with.
            self.resources
                .resize(self.resources.len().max(write + 1), NextUp::None);
            match &mut self.resources[write] {
                NextUp::Readers((_, readers)) => {
                    for reader in readers.iter() {
                        if wakers.insert(*reader) {
                            waiting_for += 1;
                            self.task_info[*reader].tasks_to_wake_up.push(task_id);
                        }
                    }
                    self.resources[write] = NextUp::Writer(task_id);
                }
                NextUp::Writer(writer) => {
                    if wakers.insert(*writer) {
                        waiting_for += 1;
                        self.task_info[*writer].tasks_to_wake_up.push(task_id);
                    }
                    self.resources[write] = NextUp::Writer(task_id);
                }
                NextUp::None => {
                    self.resources[write] = NextUp::Writer(task_id);
                }
            }
        }

        if waiting_for == 0 {
            if exclusive_task {
                // If we're an exclusive task with no dependencies we need to run after whatever was the previous task.
                if let Some(last_task) = self.last_task {
                    self.task_info[last_task].tasks_to_wake_up.push(task_id);
                    waiting_for += 1;
                } else {
                    self.starter_tasks.push(task_id);
                }
            } else if let Some(last_exclusive_task) = self.last_exclusive_task {
                // If we're a non-exclusive task with no dependencies we need to wait
                // on the last exclusive task.
                self.task_info[last_exclusive_task]
                    .tasks_to_wake_up
                    .push(task_id);
                waiting_for += 1;
            } else {
                self.starter_tasks.push(task_id);
            }
        }

        println!("WAITING FOR: {:?}", waiting_for);

        if exclusive_task {
            self.last_exclusive_task = Some(task_id);
        }

        self.last_task = Some(task_id);

        self.task_info.push(TaskInfo {
            waiting_for,
            tasks_to_wake_up: Vec::new(),
            task,
            main_thread_task,
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
                task.main_thread_task,
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

use crate::*;

pub struct SystemScheduler {
    //  inner_scheduler: Scheduler,
    systems: Vec<SystemToBeScheduled>,
}

enum SystemType {
    Exclusive(Box<dyn FnMut(&mut World) + Send>),
    NonExclusive(Box<dyn FnMut(&World) + Send>),
}
struct SystemToBeScheduled {
    get_borrows: Box<dyn Fn(&World) -> ResourceBorrows>,
    run_system: SystemType,
    main_thread_task: bool,
}

impl SystemScheduler {
    pub fn new() -> Self {
        Self {
            //   inner_scheduler: Scheduler::new(),
            systems: Vec::new(),
        }
    }

    #[track_caller]
    fn schedule_inner<P, F: for<'a> FunctionSystem<'a, (), P> + Send + 'static + Copy>(
        &mut self,
        system: F,
        main_thread_task: bool,
    ) {
        let location = Location::caller();
        let get_borrows = Box::new(move |world: &World| match system.system_info(world) {
            Err(e) => {
                /*
                println!("ERROR: {:?}", e);
                panic!(
                    "Error getting borrows for system: {}:{:?}:{:?}",
                    location.file(),
                    location.line(),
                    location.column()
                );
                */

                // If we failed to borrow from the world assume that we're dependent on nothing.
                // This is a bad approach and instead getting our borrow information should be infallible.
                // This is a patch because single borrows can currently fail when borrowing from the world.
                ResourceBorrows {
                    writes: Vec::new(),
                    reads: Vec::new(),
                }
            }
            Ok(system_info) => system_info.borrows,
        });
        if system.exclusive() {
            println!("SCHEDULING EXCLUSIVE");

            let run_system = Box::new(move |world: &mut World| {
                println!(
                    "ABOUT TO RUN EXCLUSIVE:{}:{:?}:{:?}",
                    location.file(),
                    location.line(),
                    location.column()
                );

                system.run_exclusive(world).unwrap_or_else(|e| {
                    println!("ERROR: {:?}", e);
                    panic!(
                        "Error in system: {}:{:?}:{:?}",
                        location.file(),
                        location.line(),
                        location.column()
                    );
                });
                println!("RAN EXCLUSIVE");
            });

            self.systems.push(SystemToBeScheduled {
                get_borrows,
                run_system: SystemType::Exclusive(run_system),
                main_thread_task,
            })
        } else {
            println!("SCHEDULING NONEXCLUSIVE");

            let run_system = Box::new(move |world: &World| {
                println!(
                    "ABOUT TO RUN NONEXCLUSIVE:{}:{:?}:{:?}",
                    location.file(),
                    location.line(),
                    location.column()
                );

                system.run(world).unwrap_or_else(|e| {
                    println!("ERROR: {:?}", e);
                    panic!(
                        "Error in system: {}:{:?}:{:?}",
                        location.file(),
                        location.line(),
                        location.column()
                    );
                });
                println!("RAN NONEXCLUSIVE");
            });

            self.systems.push(SystemToBeScheduled {
                get_borrows,
                run_system: SystemType::NonExclusive(run_system),
                main_thread_task,
            })
        }
    }

    #[track_caller]
    pub fn schedule<P, F: for<'a> FunctionSystem<'a, (), P> + Send + 'static + Copy>(
        &mut self,
        system: F,
    ) {
        self.schedule_inner(system, false);
    }

    #[track_caller]
    pub fn schedule_main_thread<P, F: for<'a> FunctionSystem<'a, (), P> + Send + 'static + Copy>(
        &mut self,
        system: F,
    ) {
        self.schedule_inner(system, true);
    }

    pub fn run(self, world: Arc<RwLock<World>>) {
        let world_ref = world.read().unwrap();
        let mut inner_scheduler = Scheduler::new();

        println!("SYSTEMS COUNT: {:?}", self.systems.len());
        for system_to_be_scheduled in self.systems {
            let borrows = (system_to_be_scheduled.get_borrows)(&world_ref);

            match system_to_be_scheduled.run_system {
                SystemType::NonExclusive(mut system) => {
                    let world = world.clone();
                    inner_scheduler.add_task(
                        borrows.reads.iter().copied(),
                        borrows.writes.iter().copied(),
                        Box::new(move || {
                            let world = world.read().unwrap();
                            (system)(&world);
                        }),
                        false,
                        system_to_be_scheduled.main_thread_task,
                    )
                }
                SystemType::Exclusive(mut system) => {
                    let world = world.clone();
                    inner_scheduler.add_task(
                        borrows.reads.iter().copied(),
                        borrows.writes.iter().copied(),
                        Box::new(move || {
                            let mut world = world.write().unwrap();
                            (system)(&mut world);
                        }),
                        true,
                        system_to_be_scheduled.main_thread_task,
                    )
                }
            }
        }
        inner_scheduler.run()
    }
}

#[test]
fn schedule_systems() {
    ktasks::create_workers(3);

    let mut world = World::new();
    world.spawn((10 as i32,));

    let mut system_scheduler = SystemScheduler::new();
    system_scheduler.schedule(&|i: &i32| {
        println!("IN TASK 0");
        println!("ENDING TASK 0");
    });
    system_scheduler.schedule(&|i: &i32| {
        println!("IN TASK 1");
        std::thread::sleep(std::time::Duration::from_millis(5));
        println!("ENDING TASK 1");
    });
    system_scheduler.schedule(&|i: &mut i32| {
        println!("IN TASK 2");
        println!("ENDING TASK 2");
    });

    let world = Arc::new(RwLock::new(world));
    system_scheduler.run(world);
    ktasks::run_current_thread_tasks();
    std::thread::sleep(std::time::Duration::from_millis(30));
}

#[test]
fn exclusive_system() {
    ktasks::create_workers(3);

    let mut world = World::new();
    world.spawn((10 as i32,));

    let mut system_scheduler = SystemScheduler::new();
    system_scheduler.schedule(&|i: &i32| {
        println!("IN TASK 0");
        println!("ENDING TASK 0");
    });
    system_scheduler.schedule(&|world: ExclusiveWorld| {
        println!("IN TASK 1");
        std::thread::sleep(std::time::Duration::from_millis(5));
        println!("ENDING TASK 1");
    });

    // This task should run last because the ExclusiveWorld system blocks it from running.
    system_scheduler.schedule(&|i: &i32| {
        println!("IN TASK 2");
        println!("ENDING TASK 2");
    });

    let world = Arc::new(RwLock::new(world));
    system_scheduler.run(world);
    ktasks::run_current_thread_tasks();
    std::thread::sleep(std::time::Duration::from_millis(30));
}
