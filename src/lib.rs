#[macro_use] extern crate serde_derive;
extern crate serde;
extern crate serde_json;
use serde_json::error::Category;
extern crate termion;
extern crate petgraph;
extern crate num_traits;
extern crate nix;
extern crate proc_maps;

extern crate yaxpeax_arch;

extern crate yaxpeax_x86;
extern crate yaxpeax_msp430_mc;
extern crate yaxpeax_pic17;
extern crate yaxpeax_pic18;
extern crate yaxpeax_pic24;

pub mod arch;
pub mod analyses;
pub mod debug;
pub mod memory;
pub mod parts;
pub mod comment;

use yaxpeax_arch::Arch;

use std::hash::Hash;
use std::collections::HashMap;

pub trait ContextTable<'it, A: Arch, Ctx> {
    fn at(&'it self, address: &<A as Arch>::Address) -> Ctx;
}

pub trait SyntaxedRender<A, T, F> {
    fn render(&self, context: Option<&T>, function_table: &HashMap<A, F>) -> String;
}

use analyses::static_single_assignment::cytron::SSAValues;
use analyses::static_single_assignment::cytron::SSA;
pub trait SyntaxedSSARender<Architecture: Arch + SSAValues, T, F> where
    <Architecture as Arch>::Address: Eq + Hash,
    <Architecture as SSAValues>::Location: Eq + Hash,
{
    fn render_with_ssa_values(
        &self,
        address: <Architecture as Arch>::Address,
        context: Option<&T>,
        function_table: &HashMap<<Architecture as Arch>::Address, F>,
        ssa: &SSA<Architecture>) -> String;
}

enum ISA {
    PIC17,
    PIC18,
    PIC18e,
    PIC24
}

