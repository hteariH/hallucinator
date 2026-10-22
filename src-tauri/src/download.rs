use std::path::{Path, PathBuf};

/// Where an unfinished download of `dest` lives.
pub fn part_path(dest: &Path) -> PathBuf {
    let mut s = dest.as_os_str().to_owned();
    s.push(".part");
    PathBuf::from(s)
}

use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;

#[derive(Debug, PartialEq)]
pub enum DownloadError {
    Network(String),
    Io(String),
    DiskFull,
    HashMismatch,
}

impl std::fmt::Display for DownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(e) => write!(f, "Network error: {e}"),
            Self::Io(e) => write!(f, "Disk error: {e}"),
            Self::DiskFull => write!(f, "Not enough disk space"),
            Self::HashMismatch => write!(f, "The download was corrupted (checksum mismatch). Please download again."),
        }
    }
}

fn io_err(e: std::io::Error) -> DownloadError {
    // 112 = ERROR_DISK_FULL
    if e.kind() == std::io::ErrorKind::StorageFull || e.raw_os_error() == Some(112) {
        DownloadError::DiskFull
    } else {
        DownloadError::Io(e.to_string())
    }
}

pub async fn download(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    expected_sha: &str,
    mut on_progress: impl FnMut(u64),
) -> Result<(), DownloadError> {
    let part = part_path(dest);
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(io_err)?;
    }
    let mut existing = tokio::fs::metadata(&part).await.map(|m| m.len()).unwrap_or(0);

    let mut resp = send(client, url, existing).await?;
    // 416 with a leftover part: it is probably complete (killed before verify/rename).
    if existing > 0 && resp.status() == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
        match verify_and_rename(&part, dest, expected_sha).await {
            Ok(()) => {
                on_progress(existing);
                return Ok(());
            }
            Err(DownloadError::HashMismatch) => {} // part deleted; restart from 0 once
            Err(e) => return Err(e),
        }
        existing = 0;
        resp = send(client, url, 0).await?;
    }
    let status = resp.status();
    if !status.is_success() {
        return Err(DownloadError::Network(format!("HTTP {status}")));
    }
    // A server that ignores Range sends the whole file with 200: start over.
    let resumed = existing > 0 && status == reqwest::StatusCode::PARTIAL_CONTENT;
    let mut done = if resumed { existing } else { 0 };

    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(resumed)
        .truncate(!resumed)
        .open(&part)
        .await
        .map_err(io_err)?;
    let mut body = resp.bytes_stream();
    while let Some(chunk) = body.next().await {
        let chunk = chunk.map_err(|e| DownloadError::Network(e.to_string()))?;
        file.write_all(&chunk).await.map_err(io_err)?;
        done += chunk.len() as u64;
        on_progress(done);
    }
    file.flush().await.map_err(io_err)?;
    drop(file);

    verify_and_rename(&part, dest, expected_sha).await
}

async fn send(client: &reqwest::Client, url: &str, from: u64) -> Result<reqwest::Response, DownloadError> {
    let mut req = client.get(url);
    if from > 0 {
        req = req.header(reqwest::header::RANGE, format!("bytes={from}-"));
    }
    req.send().await.map_err(|e| DownloadError::Network(e.to_string()))
}

/// Hashes the part file; on a match renames it to `dest`, otherwise deletes it.
async fn verify_and_rename(part: &Path, dest: &Path, expected_sha: &str) -> Result<(), DownloadError> {
    let p = part.to_path_buf();
    let actual = tokio::task::spawn_blocking(move || crate::models::sha256_file(&p))
        .await
        .map_err(|e| DownloadError::Io(e.to_string()))?
        .map_err(io_err)?;
    if !actual.eq_ignore_ascii_case(expected_sha) {
        let _ = tokio::fs::remove_file(part).await;
        return Err(DownloadError::HashMismatch);
    }
    tokio::fs::rename(part, dest).await.map_err(io_err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    struct TestServer {
        url: String,
        /// The `Range` start each request asked for (None = no Range header).
        ranges: Arc<Mutex<Vec<Option<u64>>>>,
    }

    /// Serves `data` over HTTP/1.1. `honor_range` answers Range with 206.
    /// `cut_first_at` makes the first response send only that many body bytes, then hang up.
    fn serve(data: Vec<u8>, honor_range: bool, cut_first_at: Option<usize>) -> TestServer {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/model.gguf", listener.local_addr().unwrap());
        let ranges = Arc::new(Mutex::new(Vec::new()));
        let seen = ranges.clone();
        std::thread::spawn(move || {
            for (i, conn) in listener.incoming().enumerate() {
                let Ok(mut conn) = conn else { break };
                let mut buf = [0u8; 4096];
                let n = conn.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_ascii_lowercase();
                let range = req.lines().find_map(|l| {
                    l.strip_prefix("range: bytes=")
                        .and_then(|r| r.trim().trim_end_matches('-').parse::<u64>().ok())
                });
                seen.lock().unwrap().push(range);
                if honor_range && range.is_some_and(|r| r as usize >= data.len()) {
                    let _ = conn.write_all(b"HTTP/1.1 416 Range Not Satisfiable
Content-Length: 0
Connection: close

");
                    continue;
                }
                let start = if honor_range { range.unwrap_or(0) as usize } else { 0 };
                let body = &data[start..];
                let status = if start > 0 { "206 Partial Content" } else { "200 OK" };
                let head = format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                let _ = conn.write_all(head.as_bytes());
                let send = match cut_first_at { Some(cut) if i == 0 => &body[..cut], _ => body };
                let _ = conn.write_all(send);
            }
        });
        TestServer { url, ranges }
    }

    fn data() -> (Vec<u8>, String) {
        let d: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
        let sha = hex::encode(Sha256::digest(&d));
        (d, sha)
    }

    #[tokio::test]
    async fn downloads_verifies_and_renames() {
        let (d, sha) = data();
        let srv = serve(d.clone(), true, None);
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("m").join("model.gguf");
        let mut last = 0;
        download(&reqwest::Client::new(), &srv.url, &dest, &sha, |done| last = done).await.unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), d);
        assert!(!part_path(&dest).exists());
        assert_eq!(last, d.len() as u64);
    }

    #[tokio::test]
    async fn resumes_after_network_drop() {
        let (d, sha) = data();
        let srv = serve(d.clone(), true, Some(50_000));
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("model.gguf");
        let client = reqwest::Client::new();
        let first = download(&client, &srv.url, &dest, &sha, |_| {}).await;
        assert!(matches!(first, Err(DownloadError::Network(_))), "{first:?}");
        assert_eq!(std::fs::metadata(part_path(&dest)).unwrap().len(), 50_000);
        download(&client, &srv.url, &dest, &sha, |_| {}).await.unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), d);
        assert_eq!(*srv.ranges.lock().unwrap(), vec![None, Some(50_000)]);
    }

    #[tokio::test]
    async fn restarts_when_server_ignores_range() {
        let (d, sha) = data();
        let srv = serve(d.clone(), false, None);
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("model.gguf");
        std::fs::write(part_path(&dest), &d[..1000]).unwrap();
        download(&reqwest::Client::new(), &srv.url, &dest, &sha, |_| {}).await.unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), d);
    }

    #[tokio::test]
    async fn complete_part_with_416_is_verified_and_renamed() {
        let (d, sha) = data();
        let srv = serve(d.clone(), true, None);
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("model.gguf");
        std::fs::write(part_path(&dest), &d).unwrap();
        let mut last = 0;
        download(&reqwest::Client::new(), &srv.url, &dest, &sha, |done| last = done).await.unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), d);
        assert!(!part_path(&dest).exists());
        assert_eq!(last, d.len() as u64);
        assert_eq!(*srv.ranges.lock().unwrap(), vec![Some(d.len() as u64)]);
    }

    #[tokio::test]
    async fn garbage_part_with_416_restarts_from_zero() {
        let (d, sha) = data();
        let srv = serve(d.clone(), true, None);
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("model.gguf");
        std::fs::write(part_path(&dest), vec![7u8; d.len()]).unwrap();
        download(&reqwest::Client::new(), &srv.url, &dest, &sha, |_| {}).await.unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), d);
        assert!(!part_path(&dest).exists());
        assert_eq!(*srv.ranges.lock().unwrap(), vec![Some(d.len() as u64), None]);
    }

    #[tokio::test]
    async fn hash_mismatch_deletes_part() {
        let (d, _) = data();
        let srv = serve(d, true, None);
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("model.gguf");
        let r = download(&reqwest::Client::new(), &srv.url, &dest, &"0".repeat(64), |_| {}).await;
        assert_eq!(r, Err(DownloadError::HashMismatch));
        assert!(!dest.exists());
        assert!(!part_path(&dest).exists());
    }
}
