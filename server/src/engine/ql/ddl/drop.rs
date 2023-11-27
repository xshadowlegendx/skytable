/*
 * Created on Wed Feb 01 2023
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

use crate::engine::{
    core::EntityIDRef,
    error::{QueryError, QueryResult},
    ql::{
        ast::{QueryData, State},
        lex::{Ident, Token},
    },
};

#[derive(Debug, PartialEq)]
/// A generic representation of `drop` query
pub struct DropSpace<'a> {
    pub(in crate::engine) space: Ident<'a>,
    pub(in crate::engine) force: bool,
}

impl<'a> DropSpace<'a> {
    #[inline(always)]
    /// Instantiate
    pub const fn new(space: Ident<'a>, force: bool) -> Self {
        Self { space, force }
    }
    fn parse<Qd: QueryData<'a>>(state: &mut State<'a, Qd>) -> QueryResult<DropSpace<'a>> {
        if state.cursor_is_ident() {
            let ident = state.fw_read();
            // should we force drop?
            let force = state.cursor_rounded_eq(Token::Ident(Ident::from("force")));
            state.cursor_ahead_if(force);
            // either `force` or nothing
            return Ok(DropSpace::new(
                unsafe {
                    // UNSAFE(@ohsayan): Safe because the if predicate ensures that tok[0] (relative) is indeed an ident
                    ident.uck_read_ident()
                },
                force,
            ));
        }
        Err(QueryError::QLInvalidSyntax)
    }
}

#[derive(Debug, PartialEq)]
pub struct DropModel<'a> {
    pub(in crate::engine) entity: EntityIDRef<'a>,
    pub(in crate::engine) force: bool,
}

impl<'a> DropModel<'a> {
    #[inline(always)]
    pub fn new(entity: EntityIDRef<'a>, force: bool) -> Self {
        Self { entity, force }
    }
    fn parse<Qd: QueryData<'a>>(state: &mut State<'a, Qd>) -> QueryResult<Self> {
        let e = state.try_entity_ref_result()?;
        let force = state.cursor_rounded_eq(Token::Ident(Ident::from("force")));
        state.cursor_ahead_if(force);
        Ok(DropModel::new(e, force))
    }
}

mod impls {
    use {
        super::{DropModel, DropSpace},
        crate::engine::{
            error::QueryResult,
            ql::ast::{traits::ASTNode, QueryData, State},
        },
    };
    impl<'a> ASTNode<'a> for DropModel<'a> {
        const MUST_USE_FULL_TOKEN_RANGE: bool = true;
        const VERIFIES_FULL_TOKEN_RANGE_USAGE: bool = false;
        fn __base_impl_parse_from_state<Qd: QueryData<'a>>(
            state: &mut State<'a, Qd>,
        ) -> QueryResult<Self> {
            Self::parse(state)
        }
    }
    impl<'a> ASTNode<'a> for DropSpace<'a> {
        const MUST_USE_FULL_TOKEN_RANGE: bool = true;
        const VERIFIES_FULL_TOKEN_RANGE_USAGE: bool = false;
        fn __base_impl_parse_from_state<Qd: QueryData<'a>>(
            state: &mut State<'a, Qd>,
        ) -> QueryResult<Self> {
            Self::parse(state)
        }
    }
}
