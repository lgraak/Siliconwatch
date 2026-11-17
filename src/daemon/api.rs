//! The opt-in API supervisor. Acquisition ownership stays in the existing worker.

use super::*;
use crate::local_api::{
    self,
    wire::{self, ApiError, Budget},
};
use std::sync::TryLockError;

fn snapshot(
    reader: &CacheReader,
    control: &Control,
    instance: &str,
    threshold: Duration,
    budget: &Budget<'_>,
) -> Result<Vec<u8>, ApiError> {
    budget.check()?;
    // Never wait behind publication, and never invert the established lock order.
    let mut view = reader
        .try_read()
        .map_err(|_| ApiError::ServiceUnavailable)?
        .ok_or(ApiError::CacheBusy)?;
    let state = match control.state.try_lock() {
        Ok(state) => state,
        Err(TryLockError::WouldBlock) => return Err(ApiError::CacheBusy),
        Err(TryLockError::Poisoned(_)) => return Err(ApiError::ServiceUnavailable),
    };
    let phase = match state.phase {
        Phase::Starting => "starting",
        Phase::Running => "running",
        Phase::Stopping => "stopping",
        Phase::Stopped => "stopped",
    };
    let assessment = view.assess().map_err(|_| ApiError::ServiceUnavailable)?;
    drop(state);
    // Only bounded memory encoding occurs under the cache guard. The returned
    // bytes own their storage; neither guard can escape into socket I/O.
    wire::snapshot(instance, phase, assessment, threshold, budget)
}

struct ApiWorker {
    stop: Arc<local_api::Stop>,
    handle: JoinHandle<io::Result<()>>,
}

fn guarded_api_worker(
    control: &Control,
    close: impl FnOnce(),
    run: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    // The run body releases its endpoint, stream and borrowed views before
    // this guard wakes the coordinator, including during unexpected unwinding.
    let _exit = OnExit(Some(|| {
        let _ = control.stop();
        close();
    }));
    run()
}

fn coordinate(
    control: &Control,
    reader: &CacheReader,
    api: ApiWorker,
    acquisition: io::Result<JoinHandle<io::Result<()>>>,
    wait: impl FnOnce(),
    close: impl FnOnce(),
    out: &mut impl Write,
) -> io::Result<()> {
    // A failed acquisition spawn must skip the signal wait but still cancel
    // and join the already-created API worker before marking Stopped.
    let mut coordinator = if acquisition.is_ok() {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(wait))
            .map_err(|_| io::Error::other("daemon signal wait panicked"))
    } else {
        Ok(())
    };
    append_error(&mut coordinator, control.stop());
    close();
    append_error(&mut coordinator, api.stop.request());
    let api_result = api
        .handle
        .join()
        .unwrap_or_else(|_| Err(io::Error::other("daemon API worker panicked")));
    append_error(&mut coordinator, api_result);

    // API resources are closed and joined before an admitted hardware read
    // can block the acquisition join. No cache view is held here.
    match acquisition {
        Ok(handle) => finish(control, reader, handle, coordinator, out),
        Err(error) => {
            let mut result = Err(error);
            append_error(&mut result, coordinator);
            report_stopped(control, reader, result, out)
        }
    }
}

pub(super) fn run(root: &Path, options: Options) -> io::Result<()> {
    let mut signals = Signals::new([SIGINT, SIGTERM])?;
    let close = signals.handle();
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|path| !path.is_empty())
        .ok_or_else(|| io::Error::other("--local-api requires XDG_RUNTIME_DIR"))?;
    let prepared = local_api::Prepared::new(Path::new(&runtime))?;
    let (owner, reader) =
        snapshot_cache::new(root, options.stale_after).map_err(io::Error::other)?;
    let control = Arc::new(Control::new());
    let stop = Arc::new(local_api::Stop::new());

    let api_control = Arc::clone(&control);
    let api_stop = Arc::clone(&stop);
    let api_close = close.clone();
    let api_reader = reader.clone();
    let api_handle = thread::Builder::new()
        .name("siliconwatch-local-api".into())
        .spawn(move || {
            guarded_api_worker(
                &api_control,
                || api_close.close(),
                || {
                    prepared.run(api_stop, |instance, budget| {
                        let result = snapshot(
                            &api_reader,
                            &api_control,
                            instance,
                            options.stale_after,
                            budget,
                        );
                        if matches!(result, Err(ApiError::ServiceUnavailable)) {
                            // snapshot() has already released all guards. Close
                            // admission immediately, even if error transmission fails.
                            let _ = api_control.stop();
                            api_close.close();
                        }
                        result
                    })
                },
            )
        });
    let api_handle = match api_handle {
        Ok(handle) => handle,
        Err(error) => {
            // spawn drops its closure (and Prepared) on failure. There is no
            // thread to join and no acquisition has been admitted.
            drop(owner);
            close.close();
            let mut result = Err(error);
            append_error(&mut result, control.stop());
            append_error(&mut result, control.stopped());
            return result;
        }
    };
    let worker_control = Arc::clone(&control);
    let worker_close = close.clone();
    let acquisition = thread::Builder::new()
        .name("siliconwatch-acquisition".into())
        .spawn(move || {
            worker(
                &worker_control,
                options,
                owner,
                || worker_close.close(),
                || {
                    let mut out = io::stderr().lock();
                    writeln!(out, "SiliconWatch daemon running")?;
                    out.flush()
                },
            )
        });
    let result = coordinate(
        &control,
        &reader,
        ApiWorker {
            stop,
            handle: api_handle,
        },
        acquisition,
        || {
            let _signal = signals.forever().next();
        },
        || close.close(),
        &mut io::stderr(),
    );
    drop(signals); // Keep registrations alive through both joins.
    result
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod output_tests;
