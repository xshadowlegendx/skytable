/*
 * Created on Mon Feb 27 2023
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

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, PartialOrd, Ord, sky_macros::EnumMethods)]
pub enum TagClass {
    Bool = 0,
    UnsignedInt = 1,
    SignedInt = 2,
    Float = 3,
    Bin = 4,
    Str = 5,
    List = 6,
}

impl TagClass {
    pub const fn try_from_raw(v: u8) -> Option<Self> {
        if v > Self::MAX {
            return None;
        }
        Some(unsafe { Self::from_raw(v) })
    }
    pub const unsafe fn from_raw(v: u8) -> Self {
        core::mem::transmute(v)
    }
    pub const fn tag_unique(&self) -> TagUnique {
        [
            TagUnique::Illegal,
            TagUnique::UnsignedInt,
            TagUnique::SignedInt,
            TagUnique::Illegal,
            TagUnique::Bin,
            TagUnique::Str,
            TagUnique::Illegal,
        ][self.value_word()]
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, PartialOrd, Ord, sky_macros::EnumMethods)]
pub enum TagSelector {
    Bool = 0,
    UInt8 = 1,
    UInt16 = 2,
    UInt32 = 3,
    UInt64 = 4,
    SInt8 = 5,
    SInt16 = 6,
    SInt32 = 7,
    SInt64 = 8,
    Float32 = 9,
    Float64 = 10,
    Bin = 11,
    Str = 12,
    List = 13,
}

impl TagSelector {
    pub const fn into_full(self) -> FullTag {
        FullTag::new(self.tag_class(), self, self.tag_unique())
    }
    pub const unsafe fn from_raw(v: u8) -> Self {
        core::mem::transmute(v)
    }
    pub const fn tag_unique(&self) -> TagUnique {
        [
            TagUnique::Illegal,
            TagUnique::UnsignedInt,
            TagUnique::UnsignedInt,
            TagUnique::UnsignedInt,
            TagUnique::UnsignedInt,
            TagUnique::SignedInt,
            TagUnique::SignedInt,
            TagUnique::SignedInt,
            TagUnique::SignedInt,
            TagUnique::Illegal,
            TagUnique::Illegal,
            TagUnique::Bin,
            TagUnique::Str,
            TagUnique::Illegal,
        ][self.value_word()]
    }
    pub const fn tag_class(&self) -> TagClass {
        [
            TagClass::Bool,
            TagClass::UnsignedInt,
            TagClass::UnsignedInt,
            TagClass::UnsignedInt,
            TagClass::UnsignedInt,
            TagClass::SignedInt,
            TagClass::SignedInt,
            TagClass::SignedInt,
            TagClass::SignedInt,
            TagClass::Float,
            TagClass::Float,
            TagClass::Bin,
            TagClass::Str,
            TagClass::List,
        ][self.value_word()]
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, PartialOrd, Ord, sky_macros::EnumMethods)]
pub enum TagUnique {
    UnsignedInt = 0,
    SignedInt = 1,
    Bin = 2,
    Str = 3,
    Illegal = 0xFF,
}

impl TagUnique {
    pub const fn is_unique(&self) -> bool {
        self.value_u8() != Self::Illegal.value_u8()
    }
    pub const fn try_from_raw(raw: u8) -> Option<Self> {
        if raw > 3 {
            return None;
        }
        Some(unsafe { core::mem::transmute(raw) })
    }
}

pub trait DataTag {
    const BOOL: Self;
    const UINT: Self;
    const SINT: Self;
    const FLOAT: Self;
    const BIN: Self;
    const STR: Self;
    const LIST: Self;
    fn tag_class(&self) -> TagClass;
    fn tag_selector(&self) -> TagSelector;
    fn tag_unique(&self) -> TagUnique;
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct FullTag {
    class: TagClass,
    selector: TagSelector,
    unique: TagUnique,
}

impl FullTag {
    const fn new(class: TagClass, selector: TagSelector, unique: TagUnique) -> Self {
        Self {
            class,
            selector,
            unique,
        }
    }
    pub const fn new_uint(selector: TagSelector) -> Self {
        Self::new(TagClass::UnsignedInt, selector, TagUnique::UnsignedInt)
    }
    pub const fn new_sint(selector: TagSelector) -> Self {
        Self::new(TagClass::SignedInt, selector, TagUnique::SignedInt)
    }
    pub const fn new_float(selector: TagSelector) -> Self {
        Self::new(TagClass::Float, selector, TagUnique::Illegal)
    }
}

macro_rules! fulltag {
    ($class:ident, $selector:ident, $unique:ident) => {
        FullTag::new(TagClass::$class, TagSelector::$selector, TagUnique::$unique)
    };
    ($class:ident, $selector:ident) => {
        fulltag!($class, $selector, Illegal)
    };
}

impl DataTag for FullTag {
    const BOOL: Self = fulltag!(Bool, Bool);
    const UINT: Self = fulltag!(UnsignedInt, UInt64, UnsignedInt);
    const SINT: Self = fulltag!(SignedInt, SInt64, SignedInt);
    const FLOAT: Self = fulltag!(Float, Float64);
    const BIN: Self = fulltag!(Bin, Bin, Bin);
    const STR: Self = fulltag!(Str, Str, Str);
    const LIST: Self = fulltag!(List, List);
    fn tag_class(&self) -> TagClass {
        self.class
    }
    fn tag_selector(&self) -> TagSelector {
        self.selector
    }
    fn tag_unique(&self) -> TagUnique {
        self.unique
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CUTag {
    class: TagClass,
    unique: TagUnique,
}

impl PartialEq for CUTag {
    fn eq(&self, other: &Self) -> bool {
        self.class == other.class
    }
}

macro_rules! cutag {
    ($class:ident, $unique:ident) => {
        CUTag::new(TagClass::$class, TagUnique::$unique)
    };
    ($class:ident) => {
        CUTag::new(TagClass::$class, TagUnique::Illegal)
    };
}

impl CUTag {
    pub const fn new(class: TagClass, unique: TagUnique) -> Self {
        Self { class, unique }
    }
}

impl DataTag for CUTag {
    const BOOL: Self = cutag!(Bool);
    const UINT: Self = cutag!(UnsignedInt, UnsignedInt);
    const SINT: Self = cutag!(SignedInt, SignedInt);
    const FLOAT: Self = cutag!(Float);
    const BIN: Self = cutag!(Bin, Bin);
    const STR: Self = cutag!(Str, Str);
    const LIST: Self = cutag!(List);

    fn tag_class(&self) -> TagClass {
        self.class
    }

    fn tag_selector(&self) -> TagSelector {
        unimplemented!()
    }

    fn tag_unique(&self) -> TagUnique {
        self.unique
    }
}

impl From<FullTag> for CUTag {
    fn from(f: FullTag) -> Self {
        Self::new(f.tag_class(), f.tag_unique())
    }
}
