use super::{
    definitions::{R2D2Pool, RedisPool, Result, CACHE_POOL_TIMEOUT_SECONDS, JOB_QUEUE},
    errors::SynxServerError,
};
use r2d2_redis::redis::{cmd, Commands, ConnectionLike};
use std::sync::Arc;
use tokio::task;

pub struct Queue {
    redis_pool: Arc<R2D2Pool>,
}

impl RedisPool for Queue {
    fn get_pool(&self) -> &R2D2Pool {
        &self.redis_pool
    }
}

impl Queue {
    pub fn new(redis_pool: Arc<R2D2Pool>) -> Self {
        Queue { redis_pool }
    }

    pub fn dequeue_job(&self) -> Result<String> {
        let mut conn = self.get_redis_connection(CACHE_POOL_TIMEOUT_SECONDS)?;

        let (_, v) = conn
            .blpop::<&str, (String, String)>(JOB_QUEUE, 0)
            .map_err(|err| SynxServerError::DequeueJobError(err.to_string()))?;

        Ok(v)
    }

    pub async fn run_workers(&self) {
        println!("Running workers...");
        loop {
            match self.dequeue_job() {
                Ok(job) => {
                    println!("JOB: {:?}", job);
                    task::spawn(Self::process_job(job));
                }
                Err(e) => {
                    // TODO: Implement re-try logic
                    eprintln!("Error dequeuing job: {}", e);
                }
            }
        }
    }

    async fn process_job(job_data: String) {
        // Process the job here
        println!("Processing job: {}", job_data);
        // Implement your job processing logic
    }
}
