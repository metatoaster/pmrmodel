use std::time::{SystemTime, UNIX_EPOCH};

pub fn timestamp() -> anyhow::Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}
