use crate::cli::Args;
use anyhow::{anyhow, Result};

// TODO: 更新 mock_server 以兼容 hyper 1.x
// 暂时禁用以便编译通过

pub async fn run(_args: Args) -> Result<()> {
    Err(anyhow!("Mock server is temporarily disabled. Will be updated to support hyper 1.x"))
}
