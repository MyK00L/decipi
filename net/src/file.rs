use crate::message::*;

use async_lock::OnceCell;
use bitvec::bitvec;
use bitvec::prelude::BitVec;
use scc::HashMap;
use std::sync::Arc;

struct FileParts {
    enc_key: EncKey,
    present: BitVec,
    data: Vec<u8>,
}
impl FileParts {
    fn new(size: usize, enc_key: EncKey) -> Self {
        Self {
            enc_key,
            present: bitvec![0; (size+FILE_CHUNK_SIZE-1)/FILE_CHUNK_SIZE],
            data: vec![0u8; size],
        }
    }
    fn nchunks(&self) -> usize {
        (self.data.len() + FILE_CHUNK_SIZE - 1) / FILE_CHUNK_SIZE
    }
    fn is_full(&self) -> bool {
        self.nchunks() == self.present.count_ones()
    }
    fn add_chunk(&mut self, chunki: usize, data: &[u8]) {
        if !self.present[chunki] {
            self.present.set(chunki, true);
            let sl = chunki * FILE_CHUNK_SIZE;
            let sr = ((chunki + 1) * FILE_CHUNK_SIZE).min(self.data.len());
            self.data[sl..sr].copy_from_slice(data);
        }
    }
    fn add_enc_chunk(&mut self, chunki: usize, chunk: Encrypted<FileChunk>) {
        if !self.present[chunki] {
            if let Some(FileChunk(data)) = chunk.inner(&self.enc_key) {
                let sr = FILE_CHUNK_SIZE.min(self.data.len() - chunki * FILE_CHUNK_SIZE);
                self.add_chunk(chunki, &data[..sr]);
            }
        }
    }
    fn get_all(&self) -> Option<Vec<u8>> {
        if self.is_full() {
            Some(self.data.clone())
        } else {
            None
        }
    }
}
pub struct FullFile {
    data: Vec<u8>,
    enc_key: EncKey,
}
enum FilePartsError {
    NotFull,
    WrongHash,
}
impl FullFile {
    fn new(data: Vec<u8>, enc_key: EncKey) -> Self {
        Self { data, enc_key }
    }
    pub fn get_chunk(&self, chunki: usize) -> &[u8] {
        &self.data[chunki * FILE_CHUNK_SIZE..((chunki + 1) * FILE_CHUNK_SIZE).min(self.data.len())]
    }
    pub fn get_enc_chunk(&self, chunki: usize) -> Encrypted<FileChunk> {
        let chunk = self.get_chunk(chunki);
        let mut data = [0u8; FILE_CHUNK_SIZE];
        data[..chunk.len()].copy_from_slice(chunk);
        Encrypted::new(FileChunk(data), &self.enc_key)
    }
    pub fn get_all(&self) -> &[u8] {
        &self.data
    }
}

#[derive(Default)]
pub struct FileStore {
    file_parts: HashMap<FileHash, FileParts>,
    full_files: HashMap<FileHash, Arc<OnceCell<FullFile>>>,
}
impl FileStore {
    pub fn new() -> Self {
        Self {
            file_parts: HashMap::new(),
            full_files: HashMap::new(),
        }
    }
    pub async fn add_done(&self, data: Vec<u8>) -> FileHash {
        let hash = Mac(blake3::hash(&data));
        let ff = FullFile::new(data, EncKey::random());
        let _ = self
            .full_files
            .entry_async(hash)
            .await
            .or_insert(Arc::new(OnceCell::new()))
            .get()
            .set(ff)
            .await;
        hash
    }
    pub async fn add_new(&self, hash: FileHash, size: usize, enc_key: EncKey) {
        let _ = self
            .file_parts
            .insert_async(hash, FileParts::new(size, enc_key))
            .await;
    }
    pub async fn add_enc_chunk(
        &self,
        hash: FileHash,
        chunki: usize,
        piece: Encrypted<FileChunk>,
    ) -> Option<bool> {
        if let Some(mut fp) = self.file_parts.get_async(&hash).await {
            fp.get_mut().add_enc_chunk(chunki, piece);
            if fp.get().is_full() {
                let value = fp.remove();
                if hash == Mac(blake3::hash(&value.data)) {
                    let ff = FullFile::new(value.data, value.enc_key);
                    let _ = self
                        .full_files
                        .entry_async(hash)
                        .await
                        .or_insert(Arc::new(OnceCell::new()))
                        .get()
                        .set(ff)
                        .await;
                    Some(true)
                } else {
                    None
                }
            } else {
                Some(false)
            }
        } else {
            None
        }
    }
    pub async fn get_file(&self, hash: FileHash) -> Arc<OnceCell<FullFile>> {
        self.full_files
            .entry_async(hash)
            .await
            .or_insert(Arc::new(OnceCell::new()))
            .get()
            .clone()
    }
}
