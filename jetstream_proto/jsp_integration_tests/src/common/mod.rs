// Common test utilities for integration tests
#![allow(dead_code)]

pub mod test_server;
pub mod test_client;

pub use test_server::TestServer;
pub use test_client::TestClient;

use std::path::PathBuf;
use tempfile::TempDir;

/// Create a temporary test file with random content
pub fn create_test_file(size: usize) -> (TempDir, PathBuf, Vec<u8>) {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("test_file.bin");
    
    // Generate random content
    use rand::RngCore;
    let mut content = vec![0u8; size];
    rand::thread_rng().fill_bytes(&mut content);
    
    std::fs::write(&file_path, &content).expect("Failed to write test file");
    
    (temp_dir, file_path, content)
}

/// Verify that two files have identical content
pub fn assert_files_equal(path1: &std::path::Path, path2: &std::path::Path) {
    let content1 = std::fs::read(path1).expect("Failed to read file 1");
    let content2 = std::fs::read(path2).expect("Failed to read file 2");
    
    assert_eq!(
        content1.len(),
        content2.len(),
        "File sizes differ: {} vs {}",
        content1.len(),
        content2.len()
    );
    
    assert_eq!(content1, content2, "File contents differ");
}

/// Wait for a condition with timeout
pub async fn wait_for<F>(mut condition: F, timeout: std::time::Duration) -> bool
where
    F: FnMut() -> bool,
{
    let start = std::time::Instant::now();
    
    while start.elapsed() < timeout {
        if condition() {
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    
    false
}
