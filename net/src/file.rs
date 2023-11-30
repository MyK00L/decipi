use crate::message::*;

use bitvec::bitvec;
use bitvec::prelude::BitVec;
use scc::HashMap;

struct FilePieces {
    enc_key: EncKey,
    present: BitVec,
    data: Vec<u8>,
}
impl FilePieces {
    fn new_empty(size: usize, enc_key: EncKey) -> Self {
        Self {
            enc_key,
            present: bitvec![0; (size+FILE_CHUNK_SIZE-1)/FILE_CHUNK_SIZE],
            data: vec![0u8; size],
        }
    }
    fn new_from_data(data: &[u8], enc_key: EncKey) -> Self {
        Self {
            enc_key,
            present: bitvec![1; (data.len()+FILE_CHUNK_SIZE-1)/FILE_CHUNK_SIZE],
            data: data.into(),
        }
    }
    fn nchunks(&self) -> usize {
        (self.data.len() + FILE_CHUNK_SIZE - 1) / FILE_CHUNK_SIZE
    }
    fn is_done(&self) -> bool {
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
    fn get_chunk(&self, chunki: usize) -> Option<&[u8]> {
        if self.present[chunki] {
            Some(
                &self.data[chunki * FILE_CHUNK_SIZE
                    ..((chunki + 1) * FILE_CHUNK_SIZE).min(self.data.len())],
            )
        } else {
            None
        }
    }
    fn get_enc_chunk(&self, chunki: usize) -> Option<Encrypted<FileChunk>> {
        self.get_chunk(chunki).map(|chunk| {
            let mut data = [0u8; FILE_CHUNK_SIZE];
            data[..chunk.len()].copy_from_slice(chunk);
            Encrypted::new(FileChunk(data), &self.enc_key)
        })
    }
    fn get_all(&self) -> Option<Vec<u8>> {
        if self.is_done() {
            Some(self.data.clone())
        } else {
            None
        }
    }
}

#[derive(Default)]
pub struct FileStore {
    files: HashMap<FileHash, FilePieces>,
}
impl FileStore {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }
    pub async fn add_done(&self, data: &[u8]) -> FileHash {
        let hash = Mac(blake3::hash(data));
        self.files
            .entry_async(hash)
            .await
            .insert_entry(FilePieces::new_from_data(data, EncKey::random()));
        hash
    }
    pub async fn add_new(&self) {}
    pub async fn get_enc_chunk(
        &self,
        hash: FileHash,
        chunki: usize,
    ) -> anyhow::Result<Encrypted<FileChunk>> {
        self.files
            .get_async(&hash)
            .await
            .ok_or(anyhow::anyhow!("file not present"))?
            .get()
            .get_enc_chunk(chunki)
            .ok_or(anyhow::anyhow!("chunk not present"))
    }
    pub async fn add_enc_chunk(
        &self,
        hash: FileHash,
        chunki: usize,
        piece: Encrypted<FileChunk>,
    ) -> anyhow::Result<()> {
        if let Some(mut fp) = self.files.get_async(&hash).await {
            fp.get_mut().add_enc_chunk(chunki, piece);
        }
        Ok(())
    }
    pub async fn get_file(&self, hash: FileHash) -> anyhow::Result<Vec<u8>> {
        self.files
            .get_async(&hash)
            .await
            .ok_or(anyhow::anyhow!("file not present"))?
            .get()
            .get_all()
            .ok_or(anyhow::anyhow!("File not done"))
    }
}
