use crate::{State, transcribe};
use std::time::Duration;

pub async fn run(state: State) {
    while !state.shutdown.is_cancelled() {
        let pending = dispatch(&state).unwrap_or(false);
        tokio::select! {
            _ = state.shutdown.cancelled() => break,
            _ = state.wake.notified() => {},
            _ = tokio::time::sleep(Duration::from_millis(1500)), if pending => {},
        }
    }
}

fn dispatch(state: &State) -> anyhow::Result<bool> {
    let queued = state.db.queued()?;
    if queued.is_empty() {
        return Ok(false);
    }
    let settings = state.db.settings()?;
    let budget = state.hardware.live().budget(&settings);
    let cap = if budget >= 1600 && state.hardware.logical >= 8 {
        settings.max_workers
    } else {
        1
    };
    let mut active = state.active.lock().unwrap();
    if state.auxiliary.available_permits() == 0 {
        return Ok(true);
    }
    let mut pending = false;
    for job in queued {
        if active.contains_key(&job.id)
            || active.len() >= cap.min(job.plan.workers)
            || active
                .values()
                .any(|(_, _, workers)| active.len() >= *workers)
        {
            pending = true;
            break;
        }
        let reserved: u64 = active.values().map(|(_, memory, _)| memory).sum();
        if job.plan.estimated_mib > budget {
            pending = true;
            break;
        }
        let per_job_limit = job.plan.memory_mib.min(budget.saturating_sub(reserved));
        if job.plan.estimated_mib > per_job_limit {
            pending = true;
            break;
        }
        if !state.db.set_running(&job.id)? {
            continue;
        }
        let token = state.shutdown.child_token();
        active.insert(
            job.id.clone(),
            (token.clone(), per_job_limit, job.plan.workers),
        );
        let state = state.clone();
        let mut job = job;
        job.plan.memory_mib = per_job_limit;
        tokio::spawn(async move {
            let id = job.id.clone();
            if let Err(error) = transcribe::work(&state, job, &token).await
                && !token.is_cancelled()
            {
                let _ = state.db.fail(&id, &error.to_string());
            }
            state.active.lock().unwrap().remove(&id);
            state.wake.notify_one();
        });
    }
    Ok(pending)
}
