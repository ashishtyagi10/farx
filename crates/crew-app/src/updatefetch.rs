//! The GitHub side of the self-update: the worker thread that streams stage
//! messages, and the calls it makes — query the latest release version, and
//! download+install it over the running binary. Kept apart from `update`'s
//! UI/state so the network work stays self-contained on the worker thread.
use std::sync::mpsc::{channel, Receiver, Sender};

use anyhow::{anyhow, Result};
use self_update::backends::github::{ReleaseList, Update};

use crate::update::UpdateMsg;

pub(crate) const REPO_OWNER: &str = "ashishtyagi10";
pub(crate) const REPO_NAME: &str = "crew";

/// Spawn the background update worker; the returned receiver streams its stages.
pub(crate) fn spawn_worker() -> Receiver<UpdateMsg> {
    let (tx, rx) = channel();
    std::thread::spawn(move || run_update(&tx));
    rx
}

/// Worker body: check GitHub, and download+install when a newer release exists.
fn run_update(tx: &Sender<UpdateMsg>) {
    let current = env!("CARGO_PKG_VERSION");
    let _ = tx.send(UpdateMsg::Checking);
    match latest_version() {
        Ok(latest) => {
            let newer = self_update::version::bump_is_greater(current, &latest).unwrap_or(false);
            if !newer {
                let _ = tx.send(UpdateMsg::UpToDate(current.to_string()));
                return;
            }
            let _ = tx.send(UpdateMsg::Downloading(latest));
            match install(current) {
                Ok(v) => {
                    let _ = tx.send(UpdateMsg::Installed(v));
                }
                Err(e) => {
                    let _ = tx.send(UpdateMsg::Failed(short_err(&e)));
                }
            }
        }
        Err(e) => {
            let _ = tx.send(UpdateMsg::Failed(short_err(&e)));
        }
    }
}

/// A one-line, card-sized error string (first line, detail trimmed off).
fn short_err(e: &anyhow::Error) -> String {
    e.to_string()
        .lines()
        .next()
        .unwrap_or("unknown")
        .to_string()
}

/// The newest release tag on GitHub (e.g. "0.6.0"), without the `v` prefix.
pub(crate) fn latest_version() -> Result<String> {
    let releases = ReleaseList::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .build()?
        .fetch()?;
    let latest = releases
        .first()
        .ok_or_else(|| anyhow!("no releases found"))?;
    Ok(latest.version.clone())
}

/// Download the latest release for this platform and replace the running binary.
/// Returns the version now on disk. `current` is this build's version.
pub(crate) fn install(current: &str) -> Result<String> {
    let status = Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name("crew")
        .current_version(current)
        .show_download_progress(false)
        .no_confirm(true)
        .build()?
        .update()?;
    Ok(status.version().to_string())
}
