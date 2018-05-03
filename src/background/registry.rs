use serde_json;
use std::collections::HashMap;

use super::Job;
use util::CargoResult;

#[doc(hidden)]
pub type PerformFn<Env> = Box<dyn Fn(serde_json::Value, &Env) -> CargoResult<()>>;

#[derive(Default)]
#[allow(missing_debug_implementations)] // Can't derive debug
/// A registry of background jobs, used to map job types to concrege perform
/// functions at runtime.
pub struct Registry<Env> {
    job_types: HashMap<&'static str, PerformFn<Env>>,
}

impl<Env> Registry<Env> {
    /// Create a new, empty registry
    pub fn new() -> Self {
        Registry {
            job_types: Default::default(),
        }
    }

    /// Get the perform function for a given job type
    pub fn get(&self, job_type: &str) -> Option<&PerformFn<Env>> {
        self.job_types.get(job_type)
    }

    /// Register a new background job. This will override any existing
    /// registries with the same `JOB_TYPE`, if one exists.
    pub fn register<T: Job<Environment = Env>>(&mut self) {
        self.job_types.insert(T::JOB_TYPE, Box::new(|data, env| {
            let data = serde_json::from_value(data)?;
            T::perform(data, env)
        }));
    }
}
