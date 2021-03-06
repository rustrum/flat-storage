use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Header exists in any block thus it should be as small as possible.
pub const BLOCK_HEADER_SPACE_BYTES: usize = 32;

const BLOCK_HEADER_SIZE: usize = core::mem::size_of::<FlatHeader>();

#[repr(u8)]
#[derive(PartialEq, Debug)]
pub enum BlockVariant {
    /// First block in the chain, could be the last one too.
    HEAD = 1,
    /// One of the many possible middle blocks
    MID = 5,
    /// Tail block, the last one.
    TAIL = 10,
}

#[repr(C)]
#[derive(PartialEq, Debug)]
pub struct FlatHeader {
    /// Version of a header. Not needed actually, but who knows for sure.
    v: u8,
    variant: BlockVariant,
    meta_size: u16,
    payload_version: u8,
    payload_size: u32,
    blocks: u32,
    next_block: u32,
    prev_block: u32,
}

impl Default for FlatHeader {
    fn default() -> Self {
        FlatHeader {
            v: 1,
            variant: BlockVariant::HEAD,
            meta_size: 0,
            payload_version: 0,
            payload_size: 0,
            blocks: 0,
            next_block: 0,
            prev_block: 0,
        }
    }
}

#[derive(PartialEq, Debug)]
pub(crate) struct FlatMeta {
    /// This is the primary key of your object.
    payload_key: String,

    /// Some info about payload that could be cached in the memory for fast access.
    meta: Vec<MetaEntry>,
}

#[derive(PartialEq, Debug)]
pub(crate) struct MetaEntry {
    k: String,
    v: Vec<u8>,
}

/// Serialize header to bytes array of the fixed size.
/// The size of [u8] must be bigger that required to store header.
pub fn header_to_bytes(h: &FlatHeader) -> [u8; BLOCK_HEADER_SPACE_BYTES] {
    let mut buf = [0; BLOCK_HEADER_SPACE_BYTES];
    let size = core::mem::size_of::<FlatHeader>();
    let ser: [u8; BLOCK_HEADER_SIZE] = unsafe {
        core::mem::transmute_copy(h)
    };

    for i in 0..BLOCK_HEADER_SIZE {
        buf[i] = ser[i];
    }

    buf
}

/// Read header from bytes.
/// Assuming that [u8] size is bigger that space occupied by header.
pub fn header_from_bytes(buf: &[u8; BLOCK_HEADER_SPACE_BYTES]) -> FlatHeader {
    let buff = &buf[0..BLOCK_HEADER_SIZE];
    unsafe {
        core::mem::transmute_copy(&buff)
    }
}


pub(crate) fn serialize_entry(e: &MetaEntry) -> Vec<u8> {
    let kb = e.k.clone().into_bytes();
    let vb = e.v.to_vec();

    let kbs: [u8; 4] = (kb.len() as u32).to_le_bytes();
    let vbs: [u8; 4] = (vb.len() as u32).to_le_bytes();

    let mut res = Vec::with_capacity(8 + kb.len() + vb.len());
    res.extend_from_slice(&kbs);
    res.extend_from_slice(&vbs);
    res.extend_from_slice(&kb);
    res.extend_from_slice(&vb);
    res
}

/// Returns deserialized entry and number of bytes that was read
pub(crate) fn deserialize_entry(buf: &[u8]) -> (MetaEntry, usize) {
    let mut bytes_read = 8usize;
    let kbs = &buf[0..4];
    let vbs = &buf[4..8];

    let kbl = u32::from_le_bytes(kbs.try_into().unwrap()) as usize;
    let vbl = u32::from_le_bytes(vbs.try_into().unwrap()) as usize;

    bytes_read += kbl + vbl;

    if bytes_read > buf.len() {
        panic!("Expecting to read more bytes than available in the input buffer");
    }

    let kb: &[u8] = &buf[8..(kbl + 8)];
    let vb: &[u8] = &buf[(8 + kbl)..(8 + kbl + vbl)];

    let key = String::from_utf8_lossy(kb);
    let value = Vec::from(vb);

    (
        MetaEntry {
            k: key.to_string(),
            v: value,
        },
        bytes_read
    )
}


#[cfg(test)]
mod tests {
    use crate::header::{BLOCK_HEADER_SIZE, BLOCK_HEADER_SPACE_BYTES, BlockVariant, deserialize_entry, FlatHeader, header_from_bytes, header_to_bytes, MetaEntry, serialize_entry};

    #[test]
    fn header_size() {
        assert!(BLOCK_HEADER_SPACE_BYTES > BLOCK_HEADER_SIZE);
    }

    #[test]
    fn header_read_write() {
        let fh1 = FlatHeader::default();
        let fh2 = FlatHeader { v: 2, variant: BlockVariant::TAIL, blocks: 10, prev_block: 10, ..FlatHeader::default() };

        let fh1_bts = header_to_bytes(&fh1);
        let fh2_bts = header_to_bytes(&fh2);

        let fh1_deser = header_from_bytes(&fh1_bts);
        let fh2_deser = header_from_bytes(&fh2_bts);

        assert_eq!(fh1, fh1_deser);
        assert_eq!(fh2, fh2_deser);

        let fh1_bts2 = header_to_bytes(&fh1_deser);
        let fh2_bts2 = header_to_bytes(&fh2_deser);

        assert_eq!(fh1_bts, fh1_bts2);
        assert_eq!(fh2_bts, fh2_bts2);
    }

    #[test]
    fn meta_entry_ser_deser() {
        let m1 = MetaEntry {
            k: "ns:object-1".to_string(),
            v: vec![1u8, 2u8, 3u8, 4u8, 5u8],
        };

        let m1_vec = serialize_entry(&m1);

        let (m1_deser, m1_bytes) = deserialize_entry(&m1_vec);

        assert_eq!(m1_bytes, m1_vec.len());
        assert_eq!(m1, m1_deser);

        let mut m1_vec2 = serialize_entry(&m1_deser);

        assert_eq!(m1_vec, m1_vec2);

        let mut m1_vec_long = m1_vec.clone();
        m1_vec_long.extend_from_slice(&[0,2,3,3,4,5]);
        let (m1_long_deser, m1_long_bytes) = deserialize_entry(&m1_vec_long);

        assert!(m1_vec.len() < m1_vec_long.len());
        assert_eq!(m1_long_bytes, m1_vec.len());
        assert_eq!(m1_long_deser, m1_deser);
    }
}
