use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::fs::File;
use std::io::{self, Read, Write, Seek, SeekFrom};
use std::collections::BTreeMap;
use bytes::Bytes;

/// Metadata for a file transfer
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileMetadata {
    pub filename: String,
    pub file_size: u64,
    pub mime_type: Option<String>,
    pub checksum: Option<Vec<u8>>, // SHA256
}

/// Header for each file chunk
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChunkHeader {
    pub transfer_id: u64,
    pub chunk_id: u64,
    pub offset: u64,
    pub data_len: u32,
    pub is_last: bool,
}

/// Enum to encapsulate different file transfer frames
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileTransferFrame {
    Metadata {
        transfer_id: u64,
        metadata: FileMetadata,
    },
    Chunk {
        header: ChunkHeader,
        data: Bytes,
    },
}

/// Helper to split file into chunks
pub struct FileSender {
    transfer_id: u64,
    file_path: PathBuf,
    file_size: u64,
    chunk_size: u32,
    file: File,
}

impl FileSender {
    pub fn new(transfer_id: u64, file_path: PathBuf, chunk_size: u32) -> io::Result<Self> {
        let file = File::open(&file_path)?;
        let metadata = file.metadata()?;
        let file_size = metadata.len();

        Ok(Self {
            transfer_id,
            file_path,
            file_size,
            chunk_size,
            file,
        })
    }

    pub fn get_metadata_frame(&self) -> FileTransferFrame {
        FileTransferFrame::Metadata {
            transfer_id: self.transfer_id,
            metadata: FileMetadata {
                filename: self.file_path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                file_size: self.file_size,
                mime_type: None, // Could use mime_guess crate if added
                checksum: None, // TODO: Calculate checksum
            },
        }
    }

    pub fn read_chunk(&mut self, chunk_id: u64) -> io::Result<Option<FileTransferFrame>> {
        let offset = chunk_id * self.chunk_size as u64;
        if offset >= self.file_size {
            return Ok(None);
        }

        self.file.seek(SeekFrom::Start(offset))?;

        let mut buffer = vec![0u8; self.chunk_size as usize];
        let bytes_read = self.file.read(&mut buffer)?;
        
        if bytes_read == 0 {
            return Ok(None);
        }

        buffer.truncate(bytes_read);

        let is_last = offset + bytes_read as u64 >= self.file_size;

        let header = ChunkHeader {
            transfer_id: self.transfer_id,
            chunk_id,
            offset,
            data_len: bytes_read as u32,
            is_last,
        };

        Ok(Some(FileTransferFrame::Chunk {
            header,
            data: Bytes::from(buffer),
        }))
    }
    
    pub fn total_chunks(&self) -> u64 {
        self.file_size.div_ceil(self.chunk_size as u64)
    }
}

/// Helper to reassemble chunks
pub struct FileReceiver {
    transfer_id: u64,
    metadata: Option<FileMetadata>,
    output_path: PathBuf,
    received_chunks: BTreeMap<u64, bool>, // chunk_id -> received
    file: Option<File>,
}

impl FileReceiver {
    pub fn new(transfer_id: u64, output_path: PathBuf) -> Self {
        Self {
            transfer_id,
            metadata: None,
            output_path,
            received_chunks: BTreeMap::new(),
            file: None,
        }
    }

    pub fn process_frame(&mut self, frame: FileTransferFrame) -> io::Result<()> {
        match frame {
            FileTransferFrame::Metadata { transfer_id, metadata } => {
                if transfer_id != self.transfer_id {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid transfer ID"));
                }
                
                // Initialize file
                let file = File::create(&self.output_path)?;
                file.set_len(metadata.file_size)?;
                
                self.file = Some(file);
                self.metadata = Some(metadata);
                Ok(())
            }
            FileTransferFrame::Chunk { header, data } => {
                if header.transfer_id != self.transfer_id {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid transfer ID"));
                }

                if let Some(file) = &mut self.file {
                    if data.len() as u32 != header.data_len {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, "Data length mismatch"));
                    }

                    file.seek(SeekFrom::Start(header.offset))?;
                    file.write_all(&data)?;
                    
                    self.received_chunks.insert(header.chunk_id, true);
                    Ok(())
                } else {
                    Err(io::Error::other("Metadata not received yet"))
                }
            }
        }
    }

    pub fn progress(&self) -> f32 {
        // This is a rough estimate based on chunk count, assuming equal chunk sizes (except last)
        // For exact progress, we'd need to track bytes written.
        // But since we don't know total chunks from metadata alone (unless we assume chunk size),
        // we can't easily calculate percentage without knowing expected chunk count.
        // Let's just return bytes written / total size if we tracked bytes.
        // For now, let's leave it simple.
        0.0 
    }
    
    pub fn is_complete(&self, total_chunks: u64) -> bool {
        self.received_chunks.len() as u64 == total_chunks
    }

    pub fn metadata(&self) -> Option<&FileMetadata> {
        self.metadata.as_ref()
    }

    pub fn output_path(&self) -> &PathBuf {
        &self.output_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_chunking() -> io::Result<()> {
        // Create a temporary file with some data
        let mut temp_file = NamedTempFile::new()?;
        let data = b"Hello, World! This is a test file for chunking.";
        temp_file.write_all(data)?;
        
        let file_path = temp_file.path().to_path_buf();
        let chunk_size = 10;
        
        let mut sender = FileSender::new(1, file_path, chunk_size)?;
        
        assert_eq!(sender.total_chunks(), 5); // 47 bytes / 10 = 5 chunks
        
        // Get metadata
        let metadata_frame = sender.get_metadata_frame();
        if let FileTransferFrame::Metadata { transfer_id, metadata } = metadata_frame {
            assert_eq!(transfer_id, 1);
            assert_eq!(metadata.file_size, 47);
        } else {
            panic!("Expected metadata frame");
        }
        
        // Read first chunk
        let chunk_frame = sender.read_chunk(0)?.unwrap();
        if let FileTransferFrame::Chunk { header, data } = chunk_frame {
            assert_eq!(header.chunk_id, 0);
            assert_eq!(header.offset, 0);
            assert_eq!(header.data_len, 10);
            assert_eq!(data.len(), 10);
            assert_eq!(&data[..], &b"Hello, Wor"[..]);
            assert!(!header.is_last);
        } else {
            panic!("Expected chunk frame");
        }
        
        Ok(())
    }
}
