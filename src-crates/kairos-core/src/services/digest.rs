//! 下载物完整性：计算归档摘要，供清单落盘与用户核对。
//!
//! 上游 release 目前只发归档、不发校验值，因此这里的摘要是**记录**而非**比对**：
//! 落进清单后可跨会话核对同一文件是否被替换，也为将来上游发布 `SHA256SUMS`
//! 时留出比对点。

use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::{KairosError, Result};

/// 读取块大小：大归档（百 MB 级）分块喂哈希，避免整文件进内存。
const CHUNK_BYTES: usize = 1024 * 1024;

/// 计算文件的 SHA-256（小写十六进制）。
pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)
        .map_err(|e| KairosError::io(format!("读取待校验文件失败：{e}")))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; CHUNK_BYTES];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| KairosError::io(format!("读取待校验文件失败：{e}")))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_temp(name: &str, content: &[u8]) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("kairos-digest-{name}"));
        std::fs::write(&path, content).expect("写入临时文件");
        path
    }

    #[test]
    fn sha256_matches_known_vectors() {
        // 空文件与 "abc" 的 SHA-256 是公开测试向量，用于锁定算法与编码口径
        let empty = write_temp("empty.bin", b"");
        assert_eq!(
            sha256_file(&empty).unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        let abc = write_temp("abc.bin", b"abc");
        assert_eq!(
            sha256_file(&abc).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let _ = std::fs::remove_file(empty);
        let _ = std::fs::remove_file(abc);
    }

    #[test]
    fn sha256_reads_across_chunk_boundaries() {
        // 跨块边界：内容重复两块，摘要必须与一次性喂入一致
        let content = vec![7u8; CHUNK_BYTES + 1];
        let path = write_temp("chunked.bin", &content);
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let expected = format!("{:x}", hasher.finalize());
        assert_eq!(sha256_file(&path).unwrap(), expected);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn sha256_reports_read_error_as_io() {
        // 目录可以 open、但 read 会失败（EISDIR）：覆盖「打开成功后的读取错误」路径，
        // 这也是把目录误当归档传入时的真实表现。
        let error = sha256_file(&std::env::temp_dir()).unwrap_err();
        assert_eq!(error.kind(), crate::error::ErrorKind::Io);
    }

    #[test]
    fn sha256_reports_missing_file_as_io() {
        let missing = std::env::temp_dir().join("kairos-digest-missing.bin");
        let _ = std::fs::remove_file(&missing);
        let error = sha256_file(&missing).unwrap_err();
        assert_eq!(error.kind(), crate::error::ErrorKind::Io);
    }
}
