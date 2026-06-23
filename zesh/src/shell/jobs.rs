// Job table

use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq)]
pub enum JobStatus {
    Running,
    Stopped,
    Done(i32),
}

#[derive(Debug, Clone)]
pub struct Job {
    pub id: usize,
    pub pid: i32,
    pub pgid: i32,
    pub cmd: String,
    pub status: JobStatus,
    pub disowned: bool,
}

pub struct JobTable {
    pub jobs: HashMap<usize, Job>,
    pub next_id: usize,
    pub last_pid: i32,  // $!
    pub current_job_id: Option<usize>,  // Most recent (marked with +)
    pub previous_job_id: Option<usize>, // Second most recent (marked with -)
}

impl JobTable {
    pub fn new() -> Self {
        JobTable {
            jobs: HashMap::new(),
            next_id: 1,
            last_pid: 0,
            current_job_id: None,
            previous_job_id: None,
        }
    }

    pub fn add(&mut self, pid: i32, cmd: String) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.last_pid = pid;
        self.jobs.insert(id, Job {
            id,
            pid,
            pgid: pid,
            cmd,
            status: JobStatus::Running,
            disowned: false,
        });
        // Update current/previous job tracking
        if let Some(current) = self.current_job_id {
            self.previous_job_id = Some(current);
        }
        self.current_job_id = Some(id);
        id
    }

    pub fn set_pgid(&mut self, pid: i32, pgid: i32) {
        if let Some(j) = self.find_by_pid_mut(pid) {
            j.pgid = pgid;
        }
    }

    pub fn remove(&mut self, pid: i32) {
        self.jobs.retain(|_, j| j.pid != pid);
    }

    pub fn find_by_id(&self, id: usize) -> Option<&Job> {
        self.jobs.get(&id)
    }

    pub fn find_by_id_mut(&mut self, id: usize) -> Option<&mut Job> {
        self.jobs.get_mut(&id)
    }

    pub fn find_by_pid(&self, pid: i32) -> Option<&Job> {
        self.jobs.values().find(|j| j.pid == pid)
    }

    pub fn find_by_pid_mut(&mut self, pid: i32) -> Option<&mut Job> {
        self.jobs.values_mut().find(|j| j.pid == pid)
    }

    pub fn get_last_job(&self) -> Option<&Job> {
        self.jobs.values()
            .filter(|j| !j.disowned && j.status == JobStatus::Running)
            .max_by_key(|j| j.id)
    }

    pub fn disown(&mut self, pid: i32) {
        if let Some(j) = self.find_by_pid_mut(pid) {
            j.disowned = true;
        }
        // Also remove from table
        self.jobs.retain(|_, j| j.pid != pid);
    }

    pub fn all_pids(&self) -> Vec<i32> {
        self.jobs.values()
            .filter(|j| !j.disowned && j.status == JobStatus::Running)
            .map(|j| j.pid)
            .collect()
    }

    pub fn resolve_job_spec(&self, spec: &str) -> Result<usize, String> {
        let spec = spec.trim();

        // Handle %% and %+ and bare % (current job)
        if spec == "+" || spec == "%+" || spec == "%%" || spec == "%" {
            if let Some(id) = self.current_job_id {
                if self.find_by_id(id).is_some() {
                    return Ok(id);
                }
            }
            return Err("no current job".to_string());
        }

        // Handle %- (previous job)
        if spec == "-" || spec == "%-" {
            if let Some(id) = self.previous_job_id {
                if self.find_by_id(id).is_some() {
                    return Ok(id);
                }
            }
            return Err("no previous job".to_string());
        }

        // Handle %n (job number)
        let spec_no_percent = if spec.starts_with('%') {
            &spec[1..]
        } else {
            spec
        };

        if let Ok(job_id) = spec_no_percent.parse::<usize>() {
            if self.find_by_id(job_id).is_some() {
                return Ok(job_id);
            }
            return Err(format!("no such job"));
        }

        // Handle %string (command prefix match) and %?string (substring match)
        let (search_str, substring_match) = if spec_no_percent.starts_with('?') {
            (&spec_no_percent[1..], true)
        } else {
            (spec_no_percent, false)
        };

        let mut matches = Vec::new();
        for job in self.jobs.values() {
            if substring_match {
                // %?string - substring match
                if job.cmd.contains(search_str) {
                    matches.push(job.id);
                }
            } else {
                // %string - prefix match
                if job.cmd.starts_with(search_str) {
                    matches.push(job.id);
                }
            }
        }

        match matches.len() {
            0 => Err(format!("no such job")),
            1 => Ok(matches[0]),
            _ => Err(format!("ambiguous job spec")),
        }
    }
}

use std::sync::OnceLock;
static JOBS: OnceLock<Mutex<JobTable>> = OnceLock::new();

pub fn jobs() -> std::sync::MutexGuard<'static, JobTable> {
    JOBS.get_or_init(|| Mutex::new(JobTable::new()))
        .lock()
        .expect("jobs lock poisoned")
}

pub fn try_get_jobs() -> Option<std::sync::MutexGuard<'static, JobTable>> {
    JOBS.get_or_init(|| Mutex::new(JobTable::new()))
        .try_lock()
        .ok()
}
