//! Keep device initialization and stream destruction off the caller's thread.
//! Some native drivers never return from opening a device. A timed-out opener
//! stays quarantined; it cannot hand a late stream to a new capture session.
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};
use std::time::Duration;

static OPENING: AtomicBool = AtomicBool::new(false);

pub struct InputStream {
    _stop: mpsc::Sender<()>,
    active: Arc<AtomicBool>,
}

impl Drop for InputStream {
    fn drop(&mut self) {
        self.active.store(false, Ordering::SeqCst);
    }
}

struct OpeningGuard;
impl Drop for OpeningGuard {
    fn drop(&mut self) {
        OPENING.store(false, Ordering::SeqCst);
    }
}

impl InputStream {
    pub fn open<T: 'static>(
        build: impl FnOnce(Arc<AtomicBool>) -> Result<T, String> + Send + 'static,
    ) -> Result<Self, String> {
        if OPENING.swap(true, Ordering::SeqCst) {
            return Err("An input device is still opening. Check microphone access and the selected device; if the driver does not recover, restart the app.".into());
        }
        // Move the guard into the closure so failed thread creation also releases it.
        let opening = OpeningGuard;
        Self::open_with_timeout(Duration::from_secs(15), move |active| {
            let result = build(active);
            drop(opening);
            result
        })
    }

    fn open_with_timeout<T: 'static>(
        timeout: Duration,
        build: impl FnOnce(Arc<AtomicBool>) -> Result<T, String> + Send + 'static,
    ) -> Result<Self, String> {
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let (stop_tx, stop_rx) = mpsc::channel();
        let active = Arc::new(AtomicBool::new(true));
        let worker_active = active.clone();
        std::thread::Builder::new()
            .name("audio-input".into())
            .spawn(move || match build(worker_active.clone()) {
                Ok(stream) => {
                    if worker_active.load(Ordering::SeqCst) && ready_tx.send(Ok(())).is_ok() {
                        let _ = stop_rx.recv();
                    }
                    worker_active.store(false, Ordering::SeqCst);
                    drop(stream);
                }
                Err(error) => {
                    let _ = ready_tx.send(Err(error));
                }
            })
            .map_err(|_| "Could not create the microphone worker".to_string())?;
        match ready_rx.recv_timeout(timeout) {
            Ok(Ok(())) => Ok(Self {
                _stop: stop_tx,
                active,
            }),
            Ok(Err(error)) => {
                active.store(false, Ordering::SeqCst);
                Err(error)
            }
            Err(_) => {
                active.store(false, Ordering::SeqCst);
                Err("Microphone opening timed out. Check microphone permission and the selected device. If the driver remains unresponsive, restart the app. Audio from this attempt will not be delivered.".into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Resource(mpsc::Sender<()>);
    impl Drop for Resource {
        fn drop(&mut self) {
            let _ = self.0.send(());
        }
    }

    #[test]
    fn late_device_is_cancelled_and_disposed_after_timeout() {
        let (release_tx, release_rx) = mpsc::channel();
        let (dropped_tx, dropped_rx) = mpsc::channel();
        let (active_tx, active_rx) = mpsc::channel();
        let result = InputStream::open_with_timeout(Duration::from_millis(10), move |active| {
            release_rx.recv().unwrap();
            active_tx.send(active.load(Ordering::SeqCst)).unwrap();
            Ok(Resource(dropped_tx))
        });
        assert!(matches!(result, Err(ref error) if error.contains("timed out")));
        release_tx.send(()).unwrap();
        assert!(!active_rx.recv_timeout(Duration::from_secs(1)).unwrap());
        dropped_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    }

    #[test]
    fn dropping_capture_disposes_the_stream_on_its_owner_thread() {
        let (tx, rx) = mpsc::channel();
        let stream =
            InputStream::open_with_timeout(Duration::from_secs(1), move |_| Ok(Resource(tx)))
                .unwrap();
        assert!(rx.try_recv().is_err());
        drop(stream);
        rx.recv_timeout(Duration::from_secs(1)).unwrap();
    }
}
