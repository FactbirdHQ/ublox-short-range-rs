use embassy_time::{Duration, Instant};

pub struct Timer {
    expires_at: Instant,
}

pub enum Error<E> {
    Timeout,
    Other(E),
}

impl Timer {
    pub fn after(duration: Duration) -> Self {
        Self {
            expires_at: Instant::now() + duration,
        }
    }

    pub fn with_timeout<F, R, E>(timeout: Duration, mut e: F) -> Result<R, Error<E>>
    where
        F: FnMut() -> Option<Result<R, E>>,
    {
        let timer = Timer::after(timeout);

        loop {
            if let Some(res) = e() {
                return res.map_err(Error::Other);
            }
            if timer.expires_at <= Instant::now() {
                return Err(Error::Timeout);
            }
        }
    }

    pub fn wait(self) {
        loop {
            if self.expires_at <= Instant::now() {
                break;
            }
        }
    }
}
