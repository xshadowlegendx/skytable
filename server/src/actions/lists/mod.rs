/*
 * Created on Tue Sep 07 2021
 *
 * This file is a part of Skytable
 * Skytable (formerly known as TerrabaseDB or Skybase) is a free and open-source
 * NoSQL database written by Sayan Nandan ("the Author") with the
 * vision to provide flexibility in data modelling without compromising
 * on performance, queryability or scalability.
 *
 * Copyright (c) 2021, Sayan Nandan <ohsayan@outlook.com>
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

use crate::corestore::table::DataModel;
use crate::corestore::Data;
use crate::dbnet::connection::prelude::*;
use crate::kvengine::listmap::LockedVec;
use crate::kvengine::KVTable;
use crate::resp::writer::TypedArrayWriter;

const LEN: &[u8] = "LEN".as_bytes();

macro_rules! listmap {
    ($tbl:expr, $con:expr) => {
        match $tbl.get_model_ref() {
            DataModel::KVExtListmap(lm) => lm,
            _ => return conwrite!($con, groups::WRONG_MODEL),
        }
    };
}

action! {
    /// Handle an `LSET` query for the list model
    /// Syntax: `LSET <listname> <values ...>`
    fn lset(handle: &Corestore, con: &mut T, mut act: ActionIter<'a>) {
        err_if_len_is!(act, con, lt 1);
        let table = get_tbl!(handle, con);
        let listmap = listmap!(table, con);
        let listname = unsafe { act.next_unchecked_bytes() };
        let list = listmap.kve_inner_ref();
        let did = if let Some(entry) = list.fresh_entry(listname.into()) {
            let v: Vec<Data> = act.map(Data::copy_from_slice).collect();
            entry.insert(LockedVec::new(v));
            true
        } else {
            false
        };
        if did {
            conwrite!(con, groups::OKAY)?;
        } else {
            conwrite!(con, groups::OVERWRITE_ERR)?;
        }
        Ok(())
    }

    /// Handle an `LGET` query for the list model (KVExt)
    /// ## Syntax
    /// - `LGET <mylist>` will return the full list
    /// - `LGET <mylist> LEN` will return the length of the list
    /// - `LGET <mylist> LIMIT <limit>` will return a maximum of `limit` elements
    /// - `LGET <mylist> VALUEAT <index>` will return the value at the provided index
    /// if it exists
    fn lget(handle: &Corestore, con: &mut T, mut act: ActionIter<'a>) {
        err_if_len_is!(act, con, lt 1);
        let table = get_tbl!(handle, con);
        let listmap = listmap!(table, con);
        // get the list name
        let listname = unsafe { act.next_unchecked() };
        // now let us see what we need to do
        match act.next_uppercase().as_ref() {
            None => {
                // just return everything in the list
                let items: Vec<Data> = if let Some(list) = listmap.get(listname) {
                    list.value().read().iter().cloned().collect()
                } else {
                    return conwrite!(con, groups::NIL);
                };
                let mut typed_array_writer = unsafe {
                    TypedArrayWriter::new(con, listmap.get_payload_tsymbol(), items.len())
                }.await?;
                for item in items {
                    typed_array_writer.write_element(item).await?;
                }
            }
            Some(subaction) => {
                match subaction.as_ref() {
                    LEN => {
                        if let Some(len) = listmap.len_of(listname) {
                            conwrite!(con, len)?;
                        } else {
                            conwrite!(con, groups::NIL)?;
                        }
                    }
                    _ => conwrite!(con, groups::UNKNOWN_ACTION)?,
                }
            }
        }
        Ok(())
    }
}
