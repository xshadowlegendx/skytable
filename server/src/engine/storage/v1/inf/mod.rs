/*
 * Created on Fri Aug 04 2023
 *
 * This file is a part of Skytable
 * Skytable (formerly known as TerrabaseDB or Skybase) is a free and open-source
 * NoSQL database written by Sayan Nandan ("the Author") with the
 * vision to provide flexibility in data modelling without compromising
 * on performance, queryability or scalability.
 *
 * Copyright (c) 2023, Sayan Nandan <ohsayan@outlook.com>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 *
*/

//! High level interfaces

pub mod map;
pub mod obj;
// tests
#[cfg(test)]
mod tests;

use crate::engine::{
    error::{RuntimeResult, StorageError},
    idx::{AsKey, AsValue, STIndex},
    mem::BufferedScanner,
};

type VecU8 = Vec<u8>;

pub trait DataSource {
    type Error;
    const RELIABLE_SOURCE: bool = true;
    fn has_remaining(&self, cnt: usize) -> bool;
    unsafe fn read_next_byte(&mut self) -> Result<u8, Self::Error>;
    unsafe fn read_next_block<const N: usize>(&mut self) -> Result<[u8; N], Self::Error>;
    unsafe fn read_next_u64_le(&mut self) -> Result<u64, Self::Error>;
    unsafe fn read_next_variable_block(&mut self, size: usize) -> Result<Vec<u8>, Self::Error>;
}

impl<'a> DataSource for BufferedScanner<'a> {
    type Error = ();
    fn has_remaining(&self, cnt: usize) -> bool {
        self.has_left(cnt)
    }
    unsafe fn read_next_byte(&mut self) -> Result<u8, Self::Error> {
        Ok(self.next_byte())
    }
    unsafe fn read_next_block<const N: usize>(&mut self) -> Result<[u8; N], Self::Error> {
        Ok(self.next_chunk())
    }
    unsafe fn read_next_u64_le(&mut self) -> Result<u64, Self::Error> {
        Ok(self.next_u64_le())
    }
    unsafe fn read_next_variable_block(&mut self, size: usize) -> Result<Vec<u8>, Self::Error> {
        Ok(self.next_chunk_variable(size).into())
    }
}

/*
    obj spec
*/

/// Any object that can be persisted
pub trait PersistObject {
    // const
    /// Size of the metadata region
    const METADATA_SIZE: usize;
    // types
    /// Input type for enc operations
    type InputType: Copy;
    /// Output type for dec operations
    type OutputType;
    /// Metadata type
    type Metadata;
    // pretest
    /// Pretest to see if the src has the required data for metadata dec. Defaults to the metadata size
    fn pretest_can_dec_metadata(scanner: &BufferedScanner) -> bool {
        scanner.has_left(Self::METADATA_SIZE)
    }
    /// Pretest to see if the src has the required data for object dec
    fn pretest_can_dec_object(scanner: &BufferedScanner, md: &Self::Metadata) -> bool;
    // meta
    /// metadata enc
    fn meta_enc(buf: &mut VecU8, data: Self::InputType);
    /// metadata dec
    ///
    /// ## Safety
    ///
    /// Must pass the [`PersistObject::pretest_can_dec_metadata`] assertion
    unsafe fn meta_dec(scanner: &mut BufferedScanner) -> RuntimeResult<Self::Metadata>;
    // obj
    /// obj enc
    fn obj_enc(buf: &mut VecU8, data: Self::InputType);
    /// obj dec
    ///
    /// ## Safety
    ///
    /// Must pass the [`PersistObject::pretest_can_dec_object`] assertion
    unsafe fn obj_dec(
        s: &mut BufferedScanner,
        md: Self::Metadata,
    ) -> RuntimeResult<Self::OutputType>;
    // default
    /// Default routine to encode an object + its metadata
    fn default_full_enc(buf: &mut VecU8, data: Self::InputType) {
        Self::meta_enc(buf, data);
        Self::obj_enc(buf, data);
    }
    /// Default routine to decode an object + its metadata (however, the metadata is used and not returned)
    fn default_full_dec(scanner: &mut BufferedScanner) -> RuntimeResult<Self::OutputType> {
        if !Self::pretest_can_dec_metadata(scanner) {
            return Err(StorageError::InternalDecodeStructureCorrupted.into());
        }
        let md = unsafe {
            // UNSAFE(@ohsayan): +pretest
            Self::meta_dec(scanner)?
        };
        if !Self::pretest_can_dec_object(scanner, &md) {
            return Err(StorageError::InternalDecodeStructureCorruptedPayload.into());
        }
        unsafe {
            // UNSAFE(@ohsayan): +obj pretest
            Self::obj_dec(scanner, md)
        }
    }
}

/*
    map spec
*/

/// specification for a persist map
pub trait PersistMapSpec {
    /// map type
    type MapType: STIndex<Self::Key, Self::Value>;
    /// map iter
    type MapIter<'a>: Iterator<Item = (&'a Self::Key, &'a Self::Value)>
    where
        Self: 'a;
    /// metadata type
    type EntryMD;
    /// key type (NOTE: set this to the true key type; handle any differences using the spec unless you have an entirely different
    /// wrapper type)
    type Key: AsKey;
    /// value type (NOTE: see [`PersistMapSpec::Key`])
    type Value: AsValue;
    /// coupled enc
    const ENC_COUPLED: bool;
    /// coupled dec
    const DEC_COUPLED: bool;
    // collection misc
    fn _get_iter<'a>(map: &'a Self::MapType) -> Self::MapIter<'a>;
    // collection meta
    /// pretest before jmp to routine for entire collection
    fn pretest_collection_using_size(_: &BufferedScanner, _: usize) -> bool {
        true
    }
    /// pretest before jmp to entry dec routine
    fn pretest_entry_metadata(scanner: &BufferedScanner) -> bool;
    /// pretest the src before jmp to entry data dec routine
    fn pretest_entry_data(scanner: &BufferedScanner, md: &Self::EntryMD) -> bool;
    // entry meta
    /// enc the entry meta
    fn entry_md_enc(buf: &mut VecU8, key: &Self::Key, val: &Self::Value);
    /// dec the entry meta
    /// SAFETY: ensure that all pretests have passed (we expect the caller to not be stupid)
    unsafe fn entry_md_dec(scanner: &mut BufferedScanner) -> Option<Self::EntryMD>;
    // independent packing
    /// enc key (non-packed)
    fn enc_key(buf: &mut VecU8, key: &Self::Key);
    /// enc val (non-packed)
    fn enc_val(buf: &mut VecU8, key: &Self::Value);
    /// dec key (non-packed)
    unsafe fn dec_key(scanner: &mut BufferedScanner, md: &Self::EntryMD) -> Option<Self::Key>;
    /// dec val (non-packed)
    unsafe fn dec_val(scanner: &mut BufferedScanner, md: &Self::EntryMD) -> Option<Self::Value>;
    // coupled packing
    /// entry packed enc
    fn enc_entry(buf: &mut VecU8, key: &Self::Key, val: &Self::Value);
    /// entry packed dec
    unsafe fn dec_entry(
        scanner: &mut BufferedScanner,
        md: Self::EntryMD,
    ) -> Option<(Self::Key, Self::Value)>;
}

// enc
pub mod enc {
    use super::{map, PersistMapSpec, PersistObject, VecU8};
    // obj
    #[cfg(test)]
    pub fn enc_full<Obj: PersistObject>(obj: Obj::InputType) -> Vec<u8> {
        let mut v = vec![];
        enc_full_into_buffer::<Obj>(&mut v, obj);
        v
    }
    pub fn enc_full_into_buffer<Obj: PersistObject>(buf: &mut VecU8, obj: Obj::InputType) {
        Obj::default_full_enc(buf, obj)
    }
    #[cfg(test)]
    pub fn enc_full_self<Obj: PersistObject<InputType = Obj>>(obj: Obj) -> Vec<u8> {
        enc_full::<Obj>(obj)
    }
    // dict
    pub fn enc_dict_full<PM: PersistMapSpec>(dict: &PM::MapType) -> Vec<u8> {
        let mut v = vec![];
        enc_dict_full_into_buffer::<PM>(&mut v, dict);
        v
    }
    pub fn enc_dict_full_into_buffer<PM: PersistMapSpec>(buf: &mut VecU8, dict: &PM::MapType) {
        <map::PersistMapImpl<PM> as PersistObject>::default_full_enc(buf, dict)
    }
}

// dec
pub mod dec {
    use {
        super::{map, PersistMapSpec, PersistObject},
        crate::engine::{error::RuntimeResult, mem::BufferedScanner},
    };
    // obj
    #[cfg(test)]
    pub fn dec_full<Obj: PersistObject>(data: &[u8]) -> RuntimeResult<Obj::OutputType> {
        let mut scanner = BufferedScanner::new(data);
        dec_full_from_scanner::<Obj>(&mut scanner)
    }
    pub fn dec_full_from_scanner<Obj: PersistObject>(
        scanner: &mut BufferedScanner,
    ) -> RuntimeResult<Obj::OutputType> {
        Obj::default_full_dec(scanner)
    }
    // dec
    pub fn dec_dict_full<PM: PersistMapSpec>(data: &[u8]) -> RuntimeResult<PM::MapType> {
        let mut scanner = BufferedScanner::new(data);
        dec_dict_full_from_scanner::<PM>(&mut scanner)
    }
    fn dec_dict_full_from_scanner<PM: PersistMapSpec>(
        scanner: &mut BufferedScanner,
    ) -> RuntimeResult<PM::MapType> {
        <map::PersistMapImpl<PM> as PersistObject>::default_full_dec(scanner)
    }
    pub mod utils {
        use crate::engine::{
            error::{RuntimeResult, StorageError},
            mem::BufferedScanner,
        };
        pub unsafe fn decode_string(s: &mut BufferedScanner, len: usize) -> RuntimeResult<String> {
            String::from_utf8(s.next_chunk_variable(len).to_owned())
                .map_err(|_| StorageError::InternalDecodeStructureCorruptedPayload.into())
        }
    }
}
