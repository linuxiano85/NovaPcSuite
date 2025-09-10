// Copyright 2025 linuxiano85
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::Result;
use tracing_subscriber::EnvFilter;

pub fn init_logging(verbose: bool) -> Result<()> {
    let filter = if verbose {
        EnvFilter::new("nova=debug,tower_http=debug")
    } else {
        EnvFilter::new("nova=info")
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .init();

    Ok(())
}

pub fn init_file_logging(log_file: &std::path::Path, verbose: bool) -> Result<()> {
    use std::fs::OpenOptions;

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)?;

    let filter = if verbose {
        EnvFilter::new("nova=debug")
    } else {
        EnvFilter::new("nova=info")
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(file)
        .with_ansi(false)
        .init();

    Ok(())
}
