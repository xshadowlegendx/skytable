/*
 * Created on Wed Oct 12 2022
 *
 * This file is a part of Skytable
 * Skytable (formerly known as TerrabaseDB or Skybase) is a free and open-source
 * NoSQL database written by Sayan Nandan ("the Author") with the
 * vision to provide flexibility in data modelling without compromising
 * on performance, queryability or scalability.
 *
 * Copyright (c) 2022, Sayan Nandan <ohsayan@outlook.com>
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

// TODO(@ohsayan): Change the underlying structures, there are just rudimentary ones used during integration with the QL

use super::ql::lexer::Lit;

/// A [`DataType`] represents the underlying data-type, although this enumeration when used in a collection will always
/// be of one type.
#[derive(Debug, PartialEq, Clone)]
pub enum DataType {
    /// An UTF-8 string
    String(Box<str>),
    /// Bytes
    Binary(Vec<u8>),
    /// An unsigned integer
    ///
    /// **NOTE:** This is the default evaluated type for unsigned integers by the query processor. It is the
    /// responsibility of the executor to ensure integrity checks depending on actual type width in the declared
    /// schema (if any)
    UnsignedInt(u64),
    /// A signed integer
    ///
    /// **NOTE:** This is the default evaluated type for signed integers by the query processor. It is the
    /// responsibility of the executor to ensure integrity checks depending on actual type width in the declared
    /// schema (if any)
    SignedInt(i64),
    /// A boolean
    Boolean(bool),
    /// A single-type list. Note, you **need** to keep up the invariant that the [`DataType`] disc. remains the same for all
    /// elements to ensure correctness in this specific context
    /// FIXME(@ohsayan): Try enforcing this somehow
    List(Vec<Self>),
}

enum_impls! {
    DataType => {
        String as String,
        Vec<u8> as Binary,
        u64 as UnsignedInt,
        bool as Boolean,
        Vec<Self> as List,
        &'static str as String,
    }
}

impl DataType {
    #[inline(always)]
    /// ## Safety
    ///
    /// Ensure validity of Lit::Bin
    pub(super) unsafe fn clone_from_lit(lit: &Lit) -> Self {
        match lit {
            Lit::Str(s) => DataType::String(s.clone()),
            Lit::Bool(b) => DataType::Boolean(*b),
            Lit::UnsignedInt(u) => DataType::UnsignedInt(*u),
            Lit::SignedInt(i) => DataType::SignedInt(*i),
            Lit::Bin(l) => DataType::Binary(l.as_slice().to_owned()),
        }
    }
}

impl<const N: usize> From<[DataType; N]> for DataType {
    fn from(f: [DataType; N]) -> Self {
        Self::List(f.into())
    }
}

constgrp! {
    #[derive(PartialEq, Eq, Clone, Copy)]
    pub struct DataKind: u8 {
        // primitive: integer unsigned
        UINT8 = 0,
        UINT16 = 1,
        UINT32 = 2,
        UINT64 = 3,
        // primitive: integer unsigned
        SINT8 = 4,
        SINT16 = 5,
        SINT32 = 6,
        SINT64 = 7,
        // primitive: misc
        BOOL = 8,
        // primitive: floating point
        FLOAT32 = 9,
        FLOAT64 = 10,
        // compound: flat
        STR = 11,
        STR_BX = Self::_BASE_HB | Self::STR.d(),
        BIN = 12,
        BIN_BX = Self::_BASE_HB | Self::BIN.d(),
        // compound: recursive
        LIST = 13,
    }
}
