// This implementation probably contains too many `Arc`s.
// Is there a way to reduce it?

use crate::*;
use ktasks::*;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::{collections::HashSet, sync::Mutex};
use std::{
    panic::Location,
    sync::atomic::{AtomicUsize, Ordering},
};

struct SystemToSchedule {
    get_dependencies: Box<dyn Fn(&World) -> SystemInfo>,
    system: Box<dyn FnMut(&World) + Send>,
}

struct Stage {
    systems_to_schedule: Vec<SystemToSchedule>,
    exclusive_system: Option<Box<dyn Fn(&mut World)>>,
}

impl Stage {
    fn new() -> Self {
        Self {
            systems_to_schedule: Vec::new(),
            exclusive_system: None,
        }
    }

    fn add_system(&mut self, system_to_schedule: SystemToSchedule) {
        self.systems_to_schedule.push(system_to_schedule);
    }

    fn add_exclusive_system(&mut self, exclusive_system: Box<dyn Fn(&mut World)>) {
        self.exclusive_system = Some(exclusive_system);
    }

    /// Schedules systems in this stage and then runs them
    fn run(self, world: &mut World) {
        type SystemIndex = usize;
        type ResourceIndex = usize;
        enum ResourceLock {
            Read {
                previous_write: Option<SystemIndex>,
                readers: Vec<SystemIndex>,
            },
            Write(SystemIndex),
        }

        struct TaskInfo {
            waiting_for: usize,
            tasks_to_wake_up: Vec<usize>,
        }

        let mut task_info: Vec<TaskInfo> = Vec::new();

        let mut starter_tasks = Vec::new();
        let mut resource_locks: HashMap<ResourceIndex, ResourceLock> = HashMap::new();

        for (system_index, system) in self.systems_to_schedule.iter().enumerate() {
            let mut waiting_for = 0;
            let dependencies = (system.get_dependencies)(world);

            // Used to ensure that each task only wakes up the next task once if they share multiple resources.
            let mut wakers: HashSet<usize> = HashSet::new();

            for read in dependencies.borrows.reads {
                let entry = resource_locks.entry(read);
                match entry {
                    Entry::Occupied(mut entry) => {
                        let mut new_read = None;
                        let entry = entry.get_mut();
                        match entry {
                            ResourceLock::Read {
                                previous_write,
                                readers,
                            } => {
                                if let Some(previous_write) = *previous_write {
                                    if wakers.insert(previous_write) {
                                        task_info[previous_write]
                                            .tasks_to_wake_up
                                            .push(system_index);
                                        waiting_for += 1;
                                    }
                                }
                                readers.push(system_index)
                            }
                            ResourceLock::Write(writer) => {
                                if wakers.insert(*writer) {
                                    task_info[*writer].tasks_to_wake_up.push(system_index);
                                    waiting_for += 1;
                                }

                                new_read = Some(ResourceLock::Read {
                                    previous_write: Some(*writer),
                                    readers: vec![system_index],
                                });
                            }
                        }
                        if let Some(new_read) = new_read {
                            *entry = new_read
                        }
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(ResourceLock::Read {
                            previous_write: None,
                            readers: vec![system_index],
                        });
                    }
                }
            }
            for write in dependencies.borrows.writes {
                let entry = resource_locks.entry(write);
                match entry {
                    Entry::Occupied(mut entry) => {
                        let mut new_write = None;
                        let entry = entry.get_mut();
                        match entry {
                            ResourceLock::Read { readers, .. } => {
                                for reader in readers.iter() {
                                    if wakers.insert(*reader) {
                                        waiting_for += 1;
                                        task_info[*reader].tasks_to_wake_up.push(system_index);
                                    }
                                }
                                new_write = Some(ResourceLock::Write(system_index));
                            }
                            ResourceLock::Write(writer) => {
                                if wakers.insert(*writer) {
                                    waiting_for += 1;
                                    task_info[*writer].tasks_to_wake_up.push(system_index);
                                }
                                *writer = system_index
                            }
                        }
                        if let Some(new_write) = new_write {
                            *entry = new_write
                        }
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(ResourceLock::Write(system_index));
                    }
                }
            }

            /*
            // Exclusive tasks depend on everything that came prior.
            if dependencies.exclusive {
                for resource in &resource_locks {
                    waiting_for += 1;
                    task_info[*resource.0].tasks_to_wake_up.push(system_index);
                }
            }
            */

            if waiting_for == 0 {
                starter_tasks.push(system_index);
            }

            task_info.push(TaskInfo {
                waiting_for,
                tasks_to_wake_up: Vec::new(),
            })
        }

        

        // Here the tasks need to be actually constructed to run on other threads
        // and wake their dependent tasks.
        todo!()
    }
}

pub struct Scheduler {
    current_stage: Stage,
    stages: Vec<Stage>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            current_stage: Stage::new(),
            stages: Vec::new(),
        }
    }

    pub fn schedule<P, F: for<'a> FunctionSystem<'a, (), P> + Send + 'static + Copy>(
        &mut self,
        system: F,
    ) {
        if system.exclusive() {
            self.current_stage.add_exclusive_system(todo!());
            let mut stage = Stage::new();
            std::mem::swap(&mut stage, &mut self.current_stage);
            self.stages.push(stage);
        } else {
            self.current_stage.add_system(todo!())
        }
    }

    pub fn run(&mut self) {}
}
