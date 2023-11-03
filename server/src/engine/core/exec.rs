/*
 * Created on Thu Oct 05 2023
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

use {
    crate::engine::{
        core::{dml, model::Model, space::Space},
        error::{QueryError, QueryResult},
        fractal::Global,
        net::protocol::{Response, SQuery},
        ql::{
            ast::{traits::ASTNode, InplaceData, State},
            lex::{Keyword, KeywordStmt, Token},
        },
    },
    core::ops::Deref,
};

pub async fn dispatch_to_executor<'a, 'b>(
    global: &'b Global,
    query: SQuery<'a>,
) -> QueryResult<Response> {
    let tokens =
        crate::engine::ql::lex::SecureLexer::new_with_segments(query.query(), query.params())
            .lex()?;
    let mut state = State::new_inplace(&tokens);
    let stmt = match state.read() {
        Token::Keyword(Keyword::Statement(stmt)) if state.remaining() >= 3 => *stmt,
        _ => return Err(QueryError::QLExpectedStatement),
    };
    state.cursor_ahead();
    if stmt.is_blocking() {
        run_blocking_stmt(state, stmt, global).await
    } else {
        run_nb(global, state, stmt)
    }
}

/*
    blocking exec
    ---
    trigger warning: disgusting hacks below (why can't async play nice with lifetimes :|)
*/

struct RawSlice<T> {
    t: *const T,
    l: usize,
}

unsafe impl<T: Send> Send for RawSlice<T> {}
unsafe impl<T: Sync> Sync for RawSlice<T> {}

impl<T> RawSlice<T> {
    #[inline(always)]
    unsafe fn new(t: *const T, l: usize) -> Self {
        Self { t, l }
    }
}

impl<T> Deref for RawSlice<T> {
    type Target = [T];
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe {
            // UNSAFE(@ohsayan): the caller MUST guarantee that this remains valid throughout the usage of the slice
            core::slice::from_raw_parts(self.t, self.l)
        }
    }
}

#[inline(always)]
fn call<A: ASTNode<'static> + core::fmt::Debug, T>(
    g: Global,
    tokens: RawSlice<Token<'static>>,
    f: impl FnOnce(&Global, A) -> QueryResult<T>,
) -> QueryResult<T> {
    let mut state = State::new_inplace(unsafe {
        // UNSAFE(@ohsayan): nothing to drop. all cool
        core::mem::transmute(tokens)
    });
    _call(&g, &mut state, f)
}

#[inline(always)]
fn _call<A: ASTNode<'static> + core::fmt::Debug, T>(
    g: &Global,
    state: &mut State<'static, InplaceData>,
    f: impl FnOnce(&Global, A) -> Result<T, QueryError>,
) -> QueryResult<T> {
    let cs = ASTNode::from_state(state)?;
    f(&g, cs)
}

async fn run_blocking_stmt(
    mut state: State<'_, InplaceData>,
    stmt: KeywordStmt,
    global: &Global,
) -> Result<Response, QueryError> {
    let (a, b) = (&state.current()[0], &state.current()[1]);
    let sysctl = stmt == KeywordStmt::Sysctl;
    let create = stmt == KeywordStmt::Create;
    let alter = stmt == KeywordStmt::Alter;
    let drop = stmt == KeywordStmt::Drop;
    let last_id = b.is_ident();
    let c_s = (create & Token![space].eq(a) & last_id) as u8 * 2;
    let c_m = (create & Token![model].eq(a) & last_id) as u8 * 3;
    let a_s = (alter & Token![space].eq(a) & last_id) as u8 * 4;
    let a_m = (alter & Token![model].eq(a) & last_id) as u8 * 5;
    let d_s = (drop & Token![space].eq(a) & last_id) as u8 * 6;
    let d_m = (drop & Token![model].eq(a) & last_id) as u8 * 7;
    let fc = sysctl as u8 | c_s | c_m | a_s | a_m | d_s | d_m;
    state.cursor_ahead();
    static BLK_EXEC: [fn(Global, RawSlice<Token<'static>>) -> QueryResult<()>; 8] = [
        |_, _| Err(QueryError::QLUnknownStatement), // unknown
        blocking_exec_sysctl,                       // sysctl
        |g, t| call(g, t, Space::transactional_exec_create),
        |g, t| call(g, t, Model::transactional_exec_create),
        |g, t| call(g, t, Space::transactional_exec_alter),
        |g, t| call(g, t, Model::transactional_exec_alter),
        |g, t| call(g, t, Space::transactional_exec_drop),
        |g, t| call(g, t, Model::transactional_exec_drop),
    ];
    let r = unsafe {
        // UNSAFE(@ohsayan): the only await is within this block
        let c_glob = global.clone();
        let ptr = state.current().as_ptr() as usize;
        let len = state.current().len();
        tokio::task::spawn_blocking(move || {
            let tokens = RawSlice::new(ptr as *const Token, len);
            BLK_EXEC[fc as usize](c_glob, tokens)?;
            Ok(Response::Empty)
        })
        .await
    };
    r.unwrap()
}

fn blocking_exec_sysctl(_: Global, _: RawSlice<Token<'static>>) -> QueryResult<()> {
    todo!()
}

/*
    nb exec
*/

fn run_nb(
    global: &Global,
    state: State<'_, InplaceData>,
    stmt: KeywordStmt,
) -> QueryResult<Response> {
    let stmt = stmt.value_u8() - KeywordStmt::Use.value_u8();
    static F: [fn(&Global, &mut State<'static, InplaceData>) -> QueryResult<Response>; 8] = [
        |_, _| panic!("use not implemented"),
        |_, _| panic!("inspect not implemented"),
        |_, _| panic!("describe not implemented"),
        |g, s| _call(g, s, dml::insert_resp),
        |_, _| panic!("select not implemented"),
        |g, s| _call(g, s, dml::update_resp),
        |g, s| _call(g, s, dml::delete_resp),
        |_, _| panic!("exists not implemented"),
    ];
    {
        let mut state = unsafe {
            // UNSAFE(@ohsayan): this is a lifetime issue with the token handle
            core::mem::transmute(state)
        };
        F[stmt as usize](global, &mut state)
    }
}
