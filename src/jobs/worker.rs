use crate::adapters::statuspage::StatuspageClient;
use crate::jobs::Job;
use tokio::sync::mpsc;
use tracing::{error, info};

pub struct JobWorker {
    receiver: mpsc::UnboundedReceiver<Job>,
    statuspage_client: Option<StatuspageClient>,
}

impl JobWorker {
    pub fn new(
        receiver: mpsc::UnboundedReceiver<Job>,
        statuspage_client: Option<StatuspageClient>,
    ) -> Self {
        Self {
            receiver,
            statuspage_client,
        }
    }

    pub async fn start(mut self) {
        info!("Job worker started");

        while let Some(job) = self.receiver.recv().await {
            // Spawn each job in a separate task to isolate panics and prevent worker death
            let statuspage_client = self.statuspage_client.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::process_job_static(statuspage_client, job).await {
                    error!("Job processing error: {}", e);
                }
            });
        }

        info!("Job worker stopped");
    }

    async fn process_job_static(statuspage_client: Option<StatuspageClient>, job: Job) -> Result<(), String> {
        match job {
            Job::StatuspageSync {
                incident_id,
                component_id,
                status,
                severity,
            } => {
                if let Some(client) = &statuspage_client {
                    crate::jobs::statuspage_sync::execute(
                        client,
                        incident_id,
                        component_id,
                        status,
                        severity,
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                } else {
                    // No Statuspage client configured, skip
                    info!("Statuspage not configured, skipping sync for incident {}", incident_id);
                }
            }
        }

        Ok(())
    }
}
