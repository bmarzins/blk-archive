use anyhow::Result;
use byteorder::{LittleEndian, WriteBytesExt};
use flate2::{write::ZlibEncoder, Compression};
use io::Write;
use std::io;

use crate::splitter::*;
use crate::content_sensitive_splitter::*;

//-----------------------------------------

struct DataPacker {
    offset: u32,
    packer: ZlibEncoder<Vec<u8>>,
}

impl Default for DataPacker {
    fn default() -> Self {
        Self {
            offset: 0,
            packer: ZlibEncoder::new(Vec::new(), Compression::default()),
        }
    }
}

impl DataPacker {
    fn write_iov(&mut self, iov: &IoVec) -> Result<()> {
        for v in iov {
            self.offset += v.len() as u32;
            self.packer.write(v)?;
        }

        Ok(())
    }

    fn complete(mut self) -> Result<Vec<u8>> {
        let r = self.packer.reset(Vec::new())?;
        Ok(r)
    }
}

//-----------------------------------------

pub struct SlabEntry {
    h: Hash,
    offset: u32,
}

pub struct Slab {
    blocks: Vec<SlabEntry>,
    packer: DataPacker,
}

impl Default for Slab {
    fn default() -> Self {
        Self {
            blocks: Vec::new(),
            packer: DataPacker::default(),
        }
    }
}

impl Slab {
    pub fn add_chunk(&mut self, h: Hash, iov: &IoVec) -> Result<()> {
        self.blocks.push(SlabEntry {
            h,
            offset: self.packer.offset,
        });

        self.packer.write_iov(iov)?;
        Ok(())
    }

    pub fn complete<W: Write>(mut self, w: &mut W) -> Result<Vec<SlabEntry>> {
        w.write_u64::<LittleEndian>(self.blocks.len() as u64)?;
        for b in &self.blocks {
            w.write(&b.h[..])?;
            w.write_u32::<LittleEndian>(b.offset as u32)?;
        }

        let compressed = self.packer.complete()?;
        w.write(&compressed[..])?;
        let mut blocks = Vec::new();
        std::mem::swap(&mut blocks, &mut self.blocks);
        Ok(blocks)
    }

    pub fn nr_entries(&self) -> usize {
        self.blocks.len()
    }

    pub fn entries_len(&self) -> usize {
        self.packer.offset as usize
    }
}

//-----------------------------------------
