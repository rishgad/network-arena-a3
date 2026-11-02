//! A small TinyVM assembler with forward labels, so branch targets are
//! never counted by hand.

#![allow(dead_code)]

use switch_program_sdk::wire::{
    CounterArrayIdW, MetaKeyW, PacketFieldW, PuntReasonW, RegW, RegisterArrayIdW, TableIdW,
};
use switch_program_sdk::{Instr, StageProgram};

pub struct Stage {
    instrs: Vec<Instr>,
    /// (instruction index, label) of branches still to patch.
    fix: Vec<(usize, &'static str)>,
    labels: Vec<(&'static str, u32)>,
}

pub fn r(n: u8) -> RegW {
    RegW(n)
}

impl Stage {
    pub fn new() -> Self {
        Self { instrs: Vec::new(), fix: Vec::new(), labels: Vec::new() }
    }
    fn push(&mut self, i: Instr) -> &mut Self {
        self.instrs.push(i);
        self
    }
    pub fn load(&mut self, d: u8, f: PacketFieldW) -> &mut Self {
        self.push(Instr::LoadField { dst: r(d), field: f })
    }
    pub fn loadm(&mut self, d: u8, k: u32) -> &mut Self {
        self.push(Instr::LoadMeta { dst: r(d), key: MetaKeyW(k) })
    }
    pub fn storem(&mut self, k: u32, s: u8) -> &mut Self {
        self.push(Instr::StoreMeta { key: MetaKeyW(k), src: r(s) })
    }
    pub fn konst(&mut self, d: u8, v: u64) -> &mut Self {
        self.push(Instr::Const { dst: r(d), value: v })
    }
    pub fn add(&mut self, d: u8, a: u8, b: u8) -> &mut Self {
        self.push(Instr::Add { dst: r(d), a: r(a), b: r(b) })
    }
    pub fn sub(&mut self, d: u8, a: u8, b: u8) -> &mut Self {
        self.push(Instr::Sub { dst: r(d), a: r(a), b: r(b) })
    }
    pub fn and(&mut self, d: u8, a: u8, b: u8) -> &mut Self {
        self.push(Instr::And { dst: r(d), a: r(a), b: r(b) })
    }
    pub fn xor(&mut self, d: u8, a: u8, b: u8) -> &mut Self {
        self.push(Instr::Xor { dst: r(d), a: r(a), b: r(b) })
    }
    pub fn eq(&mut self, d: u8, a: u8, b: u8) -> &mut Self {
        self.push(Instr::Eq { dst: r(d), a: r(a), b: r(b) })
    }
    pub fn lt(&mut self, d: u8, a: u8, b: u8) -> &mut Self {
        self.push(Instr::Lt { dst: r(d), a: r(a), b: r(b) })
    }
    /// `d = a * 2^k` by doubling.
    pub fn shl(&mut self, d: u8, a: u8, k: u32) -> &mut Self {
        if k == 0 {
            let z = 250;
            self.konst(z, 0).add(d, a, z);
        }
        for i in 0..k {
            if i == 0 { self.add(d, a, a) } else { self.add(d, d, d) };
        }
        self
    }
    pub fn table(&mut self, t: u32, key: u8, result_meta: u32) -> &mut Self {
        self.push(Instr::TableLookup { table_id: TableIdW(t), key_reg: r(key), result_meta: MetaKeyW(result_meta) })
    }
    pub fn rread(&mut self, a: u32, idx: u8, d: u8) -> &mut Self {
        self.push(Instr::RegisterRead { array: RegisterArrayIdW(a), index: r(idx), dst: r(d) })
    }
    pub fn rwrite(&mut self, a: u32, idx: u8, s: u8) -> &mut Self {
        self.push(Instr::RegisterWrite { array: RegisterArrayIdW(a), index: r(idx), src: r(s) })
    }
    pub fn counter(&mut self, c: u32, idx: u8, v: u8) -> &mut Self {
        self.push(Instr::CounterAdd { counter: CounterArrayIdW(c), index: r(idx), value: r(v) })
    }
    pub fn punt(&mut self, reason: u32) -> &mut Self {
        self.push(Instr::Punt { reason: PuntReasonW::Custom(reason) })
    }
    pub fn noop(&mut self) -> &mut Self {
        self.push(Instr::Noop)
    }
    /// Branch to `label` (defined later) when `cond` is nonzero.
    pub fn bif(&mut self, cond: u8, label: &'static str) -> &mut Self {
        self.fix.push((self.instrs.len(), label));
        self.push(Instr::BranchIf { cond: r(cond), target: 0 })
    }
    /// Branch to `label` unless `cond` is nonzero (uses reg 251, 252).
    pub fn bifnot(&mut self, cond: u8, label: &'static str) -> &mut Self {
        self.konst(251, 1).xor(252, cond, 251).bif(252, label)
    }
    /// Always branch (uses reg 253).
    pub fn jmp(&mut self, label: &'static str) -> &mut Self {
        self.konst(253, 1).bif(253, label)
    }
    pub fn label(&mut self, l: &'static str) -> &mut Self {
        self.labels.push((l, self.instrs.len() as u32));
        self
    }

    /// The finished stage. Every label must be defined, and after its
    /// branch; a label at the very end gets a trailing `noop`.
    pub fn build(&mut self, alu: u32, mem: u8) -> StageProgram {
        if self.labels.iter().any(|(_, at)| *at as usize == self.instrs.len()) {
            self.instrs.push(Instr::Noop);
        }
        for (at, l) in &self.fix {
            let target = self.labels.iter().find(|(n, _)| n == l).unwrap_or_else(|| panic!("label {l}")).1;
            assert!(target as usize > *at, "label {l} must come after its branch");
            if let Instr::BranchIf { target: t, .. } = &mut self.instrs[*at] {
                *t = target;
            }
        }
        StageProgram { instrs: std::mem::take(&mut self.instrs), max_alu_ops: alu, max_memory_accesses: mem }
    }
}
