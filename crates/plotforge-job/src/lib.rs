use std::collections::BTreeMap;

use plotforge_schema::{JobCost, JobFailure, JobKind, JobProgress, JobRecord, JobStatus};
use thiserror::Error;

mod usage;

pub use usage::{
    UsageKind, UsageLedger, UsageLedgerEntry, UsageLedgerError, UsageReport, usage_ledger_path,
};

pub trait JobClock {
    fn now_ms(&self) -> u64;
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SystemJobClock;

impl JobClock for SystemJobClock {
    fn now_ms(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or(0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobRequest {
    pub kind: JobKind,
    pub timeout_ms: u64,
    pub max_attempts: u32,
    pub estimated_cost_units: u64,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum JobQueueError {
    #[error("invalid job request: {0}")]
    InvalidRequest(&'static str),
    #[error("job does not exist: {0}")]
    MissingJob(String),
    #[error("cannot {action} job {id} while status is {status:?}")]
    InvalidTransition {
        id: String,
        status: JobStatus,
        action: &'static str,
    },
    #[error("job {id} reported invalid progress: {completed_units}/{total_units}")]
    InvalidProgress {
        id: String,
        completed_units: u32,
        total_units: u32,
    },
    #[error("job {0} failure is not retryable")]
    NonRetryableFailure(String),
    #[error("job {id} exhausted retry attempts: {attempt}/{max_attempts}")]
    RetryLimitReached {
        id: String,
        attempt: u32,
        max_attempts: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobQueue<C> {
    clock: C,
    next_sequence: u64,
    records: BTreeMap<String, JobRecord>,
}

impl<C> JobQueue<C>
where
    C: JobClock,
{
    pub fn new(clock: C) -> Self {
        Self {
            clock,
            next_sequence: 1,
            records: BTreeMap::new(),
        }
    }

    pub fn enqueue(&mut self, request: JobRequest) -> Result<JobRecord, JobQueueError> {
        if request.max_attempts == 0 {
            return Err(JobQueueError::InvalidRequest(
                "max_attempts must be greater than 0",
            ));
        }
        if request.timeout_ms == 0 {
            return Err(JobQueueError::InvalidRequest(
                "timeout_ms must be greater than 0",
            ));
        }

        let now_ms = self.clock.now_ms();
        let id = format!("job-{:06}", self.next_sequence);
        self.next_sequence += 1;
        let record = JobRecord {
            id: id.clone(),
            kind: request.kind,
            status: JobStatus::Queued,
            attempt: 1,
            max_attempts: request.max_attempts,
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
            started_at_ms: None,
            finished_at_ms: None,
            timeout_ms: request.timeout_ms,
            progress: JobProgress::default(),
            cost: JobCost {
                estimated_units: request.estimated_cost_units,
                spent_units: 0,
            },
            failure: None,
        };
        self.records.insert(id, record.clone());
        Ok(record)
    }

    pub fn record(&self, id: &str) -> Option<&JobRecord> {
        self.records.get(id)
    }

    pub fn records(&self) -> impl Iterator<Item = &JobRecord> {
        self.records.values()
    }

    pub fn start_next(&mut self) -> Option<JobRecord> {
        let id = self
            .records
            .values()
            .find(|record| record.status == JobStatus::Queued)
            .map(|record| record.id.clone())?;
        self.start(&id).ok()
    }

    pub fn start(&mut self, id: &str) -> Result<JobRecord, JobQueueError> {
        let now_ms = self.clock.now_ms();
        let record = self.record_mut(id)?;
        if record.status != JobStatus::Queued {
            return Err(invalid_transition(record, "start"));
        }

        record.status = JobStatus::Running;
        record.started_at_ms = Some(now_ms);
        record.updated_at_ms = now_ms;
        record.failure = None;
        Ok(record.clone())
    }

    pub fn report_progress(
        &mut self,
        id: &str,
        completed_units: u32,
        total_units: u32,
        message: impl Into<Option<String>>,
    ) -> Result<JobRecord, JobQueueError> {
        let now_ms = self.clock.now_ms();
        let record = self.record_mut(id)?;
        if record.status != JobStatus::Running {
            return Err(invalid_transition(record, "report_progress"));
        }
        if completed_units > total_units {
            return Err(JobQueueError::InvalidProgress {
                id: id.to_string(),
                completed_units,
                total_units,
            });
        }

        record.progress = JobProgress {
            completed_units,
            total_units,
            message: message.into(),
        };
        record.updated_at_ms = now_ms;
        Ok(record.clone())
    }

    pub fn succeed(&mut self, id: &str, spent_units: u64) -> Result<JobRecord, JobQueueError> {
        let now_ms = self.clock.now_ms();
        let record = self.record_mut(id)?;
        if record.status != JobStatus::Running {
            return Err(invalid_transition(record, "succeed"));
        }

        record.status = JobStatus::Succeeded;
        record.progress.completed_units = record.progress.total_units;
        record.cost.spent_units = spent_units;
        record.finished_at_ms = Some(now_ms);
        record.updated_at_ms = now_ms;
        record.failure = None;
        Ok(record.clone())
    }

    pub fn fail(&mut self, id: &str, failure: JobFailure) -> Result<JobRecord, JobQueueError> {
        let now_ms = self.clock.now_ms();
        let record = self.record_mut(id)?;
        if record.status != JobStatus::Running {
            return Err(invalid_transition(record, "fail"));
        }

        record.status = JobStatus::Failed;
        record.failure = Some(failure);
        record.finished_at_ms = Some(now_ms);
        record.updated_at_ms = now_ms;
        Ok(record.clone())
    }

    pub fn retry(&mut self, id: &str) -> Result<JobRecord, JobQueueError> {
        let now_ms = self.clock.now_ms();
        let record = self.record_mut(id)?;
        if !matches!(record.status, JobStatus::Failed | JobStatus::TimedOut) {
            return Err(invalid_transition(record, "retry"));
        }
        let failure = record
            .failure
            .as_ref()
            .ok_or_else(|| JobQueueError::NonRetryableFailure(id.to_string()))?;
        if !failure.retryable {
            return Err(JobQueueError::NonRetryableFailure(id.to_string()));
        }
        if record.attempt >= record.max_attempts {
            return Err(JobQueueError::RetryLimitReached {
                id: id.to_string(),
                attempt: record.attempt,
                max_attempts: record.max_attempts,
            });
        }

        record.attempt += 1;
        record.status = JobStatus::Queued;
        record.started_at_ms = None;
        record.finished_at_ms = None;
        record.updated_at_ms = now_ms;
        record.progress = JobProgress::default();
        record.failure = None;
        Ok(record.clone())
    }

    pub fn cancel(
        &mut self,
        id: &str,
        reason: impl Into<String>,
    ) -> Result<JobRecord, JobQueueError> {
        let now_ms = self.clock.now_ms();
        let record = self.record_mut(id)?;
        if !matches!(record.status, JobStatus::Queued | JobStatus::Running) {
            return Err(invalid_transition(record, "cancel"));
        }

        record.status = JobStatus::Canceled;
        record.failure = Some(JobFailure {
            code: "job_canceled".into(),
            message: reason.into(),
            retryable: false,
        });
        record.finished_at_ms = Some(now_ms);
        record.updated_at_ms = now_ms;
        Ok(record.clone())
    }

    pub fn expire_timeouts(&mut self) -> Vec<JobRecord> {
        let now_ms = self.clock.now_ms();
        let timed_out_ids = self
            .records
            .values()
            .filter(|record| record.status == JobStatus::Running)
            .filter(|record| {
                record
                    .started_at_ms
                    .is_some_and(|started| now_ms.saturating_sub(started) >= record.timeout_ms)
            })
            .map(|record| record.id.clone())
            .collect::<Vec<_>>();

        timed_out_ids
            .into_iter()
            .filter_map(|id| {
                let record = self.records.get_mut(&id)?;
                record.status = JobStatus::TimedOut;
                record.failure = Some(JobFailure {
                    code: "job_timeout".into(),
                    message: format!("job exceeded timeout_ms={}", record.timeout_ms),
                    retryable: true,
                });
                record.finished_at_ms = Some(now_ms);
                record.updated_at_ms = now_ms;
                Some(record.clone())
            })
            .collect()
    }

    fn record_mut(&mut self, id: &str) -> Result<&mut JobRecord, JobQueueError> {
        self.records
            .get_mut(id)
            .ok_or_else(|| JobQueueError::MissingJob(id.to_string()))
    }
}

fn invalid_transition(record: &JobRecord, action: &'static str) -> JobQueueError {
    JobQueueError::InvalidTransition {
        id: record.id.clone(),
        status: record.status.clone(),
        action,
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use plotforge_schema::{JobFailure, JobKind, JobStatus};

    use super::{JobClock, JobQueue, JobQueueError, JobRequest};

    #[derive(Clone, Debug)]
    struct FakeClock {
        now_ms: Rc<Cell<u64>>,
    }

    impl FakeClock {
        fn new(now_ms: u64) -> Self {
            Self {
                now_ms: Rc::new(Cell::new(now_ms)),
            }
        }

        fn set(&self, now_ms: u64) {
            self.now_ms.set(now_ms);
        }
    }

    impl JobClock for FakeClock {
        fn now_ms(&self) -> u64 {
            self.now_ms.get()
        }
    }

    #[test]
    fn enqueue_start_and_progress_use_injected_clock() {
        let clock = FakeClock::new(100);
        let mut queue = JobQueue::new(clock.clone());

        let queued = queue
            .enqueue(request(JobKind::ImageGeneration))
            .expect("enqueue");
        assert_eq!(queued.id, "job-000001");
        assert_eq!(queued.status, JobStatus::Queued);
        assert_eq!(queued.created_at_ms, 100);
        assert_eq!(queued.updated_at_ms, 100);

        clock.set(150);
        let running = queue.start(&queued.id).expect("start");
        assert_eq!(running.status, JobStatus::Running);
        assert_eq!(running.started_at_ms, Some(150));

        clock.set(175);
        let progress = queue
            .report_progress(&queued.id, 2, 5, Some("rendering".into()))
            .expect("progress");
        assert_eq!(progress.progress.completed_units, 2);
        assert_eq!(progress.progress.total_units, 5);
        assert_eq!(progress.progress.message.as_deref(), Some("rendering"));
        assert_eq!(progress.updated_at_ms, 175);
    }

    #[test]
    fn enqueue_rejects_invalid_request_without_silent_defaults() {
        let mut queue = JobQueue::new(FakeClock::new(0));

        let error = queue
            .enqueue(JobRequest {
                max_attempts: 0,
                ..request(JobKind::ImageGeneration)
            })
            .expect_err("zero attempts rejected");
        assert_eq!(
            error,
            JobQueueError::InvalidRequest("max_attempts must be greater than 0")
        );

        let error = queue
            .enqueue(JobRequest {
                timeout_ms: 0,
                ..request(JobKind::ImageGeneration)
            })
            .expect_err("zero timeout rejected");
        assert_eq!(
            error,
            JobQueueError::InvalidRequest("timeout_ms must be greater than 0")
        );
    }

    #[test]
    fn report_progress_rejects_completed_units_over_total() {
        let mut queue = JobQueue::new(FakeClock::new(0));
        let job = queue
            .enqueue(request(JobKind::ImageGeneration))
            .expect("enqueue");
        queue.start(&job.id).expect("start");

        let error = queue
            .report_progress(&job.id, 6, 5, Some("overreported".into()))
            .expect_err("invalid progress rejected");

        assert_eq!(
            error,
            JobQueueError::InvalidProgress {
                id: job.id,
                completed_units: 6,
                total_units: 5
            }
        );
    }

    #[test]
    fn cancel_records_explicit_failure_without_retry() {
        let clock = FakeClock::new(10);
        let mut queue = JobQueue::new(clock.clone());
        let job = queue
            .enqueue(request(JobKind::TextGeneration))
            .expect("enqueue");
        queue.start(&job.id).expect("start");

        clock.set(20);
        let canceled = queue.cancel(&job.id, "user canceled").expect("cancel");

        assert_eq!(canceled.status, JobStatus::Canceled);
        assert_eq!(canceled.finished_at_ms, Some(20));
        let failure = canceled.failure.expect("failure");
        assert_eq!(failure.code, "job_canceled");
        assert!(!failure.retryable);
    }

    #[test]
    fn retry_failed_retryable_job_resets_progress_and_increments_attempt() {
        let clock = FakeClock::new(0);
        let mut queue = JobQueue::new(clock.clone());
        let job = queue
            .enqueue(JobRequest {
                max_attempts: 2,
                ..request(JobKind::ExportPackage)
            })
            .expect("enqueue");
        queue.start(&job.id).expect("start");
        queue
            .report_progress(&job.id, 1, 3, Some("packing".into()))
            .expect("progress");
        let failed = queue
            .fail(
                &job.id,
                JobFailure {
                    code: "export_io".into(),
                    message: "temporary write failure".into(),
                    retryable: true,
                },
            )
            .expect("fail");
        assert_eq!(failed.status, JobStatus::Failed);

        clock.set(50);
        let retried = queue.retry(&job.id).expect("retry");

        assert_eq!(retried.status, JobStatus::Queued);
        assert_eq!(retried.attempt, 2);
        assert_eq!(retried.progress.completed_units, 0);
        assert_eq!(retried.progress.total_units, 0);
        assert!(retried.failure.is_none());
        assert_eq!(retried.updated_at_ms, 50);
    }

    #[test]
    fn timeout_marks_running_job_with_retryable_failure() {
        let clock = FakeClock::new(1_000);
        let mut queue = JobQueue::new(clock.clone());
        let job = queue
            .enqueue(JobRequest {
                timeout_ms: 100,
                ..request(JobKind::TtsGeneration)
            })
            .expect("enqueue");
        queue.start(&job.id).expect("start");

        clock.set(1_099);
        assert!(queue.expire_timeouts().is_empty());

        clock.set(1_100);
        let timed_out = queue.expire_timeouts();

        assert_eq!(timed_out.len(), 1);
        assert_eq!(timed_out[0].status, JobStatus::TimedOut);
        assert_eq!(timed_out[0].finished_at_ms, Some(1_100));
        let failure = timed_out[0].failure.as_ref().expect("failure");
        assert_eq!(failure.code, "job_timeout");
        assert!(failure.retryable);
    }

    #[test]
    fn retry_timed_out_job_resets_progress_and_increments_attempt() {
        let clock = FakeClock::new(1_000);
        let mut queue = JobQueue::new(clock.clone());
        let job = queue
            .enqueue(JobRequest {
                timeout_ms: 100,
                max_attempts: 2,
                ..request(JobKind::TtsGeneration)
            })
            .expect("enqueue");
        queue.start(&job.id).expect("start");

        clock.set(1_100);
        assert_eq!(queue.expire_timeouts().len(), 1);

        clock.set(1_150);
        let retried = queue.retry(&job.id).expect("retry timeout");

        assert_eq!(retried.status, JobStatus::Queued);
        assert_eq!(retried.attempt, 2);
        assert_eq!(retried.started_at_ms, None);
        assert_eq!(retried.finished_at_ms, None);
        assert!(retried.failure.is_none());
    }

    #[test]
    fn succeed_records_progress_and_spent_cost() {
        let clock = FakeClock::new(0);
        let mut queue = JobQueue::new(clock.clone());
        let job = queue
            .enqueue(JobRequest {
                estimated_cost_units: 500,
                ..request(JobKind::ImageGeneration)
            })
            .expect("enqueue");
        queue.start(&job.id).expect("start");
        queue
            .report_progress(&job.id, 1, 4, None)
            .expect("progress");

        clock.set(75);
        let completed = queue.succeed(&job.id, 425).expect("succeed");

        assert_eq!(completed.status, JobStatus::Succeeded);
        assert_eq!(completed.finished_at_ms, Some(75));
        assert_eq!(completed.cost.estimated_units, 500);
        assert_eq!(completed.cost.spent_units, 425);
        assert_eq!(completed.progress.completed_units, 4);
        assert_eq!(completed.progress.total_units, 4);
        assert!(completed.failure.is_none());
    }

    #[test]
    fn non_retryable_failure_and_exhausted_attempts_are_explicit_errors() {
        let clock = FakeClock::new(0);
        let mut queue = JobQueue::new(clock);
        let non_retryable = queue
            .enqueue(request(JobKind::ReferenceAnalysis))
            .expect("enqueue");
        queue.start(&non_retryable.id).expect("start");
        queue
            .fail(
                &non_retryable.id,
                JobFailure {
                    code: "invalid_input".into(),
                    message: "source cannot be processed".into(),
                    retryable: false,
                },
            )
            .expect("fail");

        let error = queue
            .retry(&non_retryable.id)
            .expect_err("non retryable should fail");
        assert_eq!(
            error,
            JobQueueError::NonRetryableFailure(non_retryable.id.clone())
        );

        let exhausted = queue
            .enqueue(JobRequest {
                max_attempts: 1,
                ..request(JobKind::TextGeneration)
            })
            .expect("enqueue");
        queue.start(&exhausted.id).expect("start exhausted");
        queue
            .fail(
                &exhausted.id,
                JobFailure {
                    code: "provider_timeout".into(),
                    message: "provider timed out".into(),
                    retryable: true,
                },
            )
            .expect("fail exhausted");

        let error = queue.retry(&exhausted.id).expect_err("attempts exhausted");
        assert_eq!(
            error,
            JobQueueError::RetryLimitReached {
                id: exhausted.id,
                attempt: 1,
                max_attempts: 1
            }
        );
    }

    fn request(kind: JobKind) -> JobRequest {
        JobRequest {
            kind,
            timeout_ms: 1_000,
            max_attempts: 3,
            estimated_cost_units: 0,
        }
    }
}
