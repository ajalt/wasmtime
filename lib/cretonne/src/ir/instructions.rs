//! Instruction formats and opcodes.
//!
//! The `instructions` module contains definitions for instruction formats, opcodes, and the
//! in-memory representation of IL instructions.
//!
//! A large part of this module is auto-generated from the instruction descriptions in the meta
//! directory.

use std::fmt::{self, Display, Formatter};
use std::str::FromStr;
use std::ops::{Deref, DerefMut};

use ir::{Value, Type, Ebb, JumpTable, SigRef, FuncRef, StackSlot, MemFlags};
use ir::immediates::{Imm64, Uimm8, Ieee32, Ieee64, Offset32, Uoffset32};
use ir::condcodes::*;
use ir::types;
use isa::RegUnit;

use entity_list;
use bitset::BitSet;
use ref_slice::{ref_slice, ref_slice_mut};

/// Some instructions use an external list of argument values because there is not enough space in
/// the 16-byte `InstructionData` struct. These value lists are stored in a memory pool in
/// `dfg.value_lists`.
pub type ValueList = entity_list::EntityList<Value>;

/// Memory pool for holding value lists. See `ValueList`.
pub type ValueListPool = entity_list::ListPool<Value>;

// Include code generated by `lib/cretonne/meta/gen_instr.py`. This file contains:
//
// - The `pub enum InstructionFormat` enum with all the instruction formats.
// - The `pub enum Opcode` definition with all known opcodes,
// - The `const OPCODE_FORMAT: [InstructionFormat; N]` table.
// - The private `fn opcode_name(Opcode) -> &'static str` function, and
// - The hash table `const OPCODE_HASH_TABLE: [Opcode; N]`.
//
// For value type constraints:
//
// - The `const OPCODE_CONSTRAINTS : [OpcodeConstraints; N]` table.
// - The `const TYPE_SETS : [ValueTypeSet; N]` table.
// - The `const OPERAND_CONSTRAINTS : [OperandConstraint; N]` table.
//
include!(concat!(env!("OUT_DIR"), "/opcodes.rs"));

impl Display for Opcode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", opcode_name(*self))
    }
}

impl Opcode {
    /// Get the instruction format for this opcode.
    pub fn format(self) -> InstructionFormat {
        OPCODE_FORMAT[self as usize - 1]
    }

    /// Get the constraint descriptor for this opcode.
    /// Panic if this is called on `NotAnOpcode`.
    pub fn constraints(self) -> OpcodeConstraints {
        OPCODE_CONSTRAINTS[self as usize - 1]
    }
}

// This trait really belongs in lib/reader where it is used by the `.cton` file parser, but since
// it critically depends on the `opcode_name()` function which is needed here anyway, it lives in
// this module. This also saves us from running the build script twice to generate code for the two
// separate crates.
impl FromStr for Opcode {
    type Err = &'static str;

    /// Parse an Opcode name from a string.
    fn from_str(s: &str) -> Result<Opcode, &'static str> {
        use constant_hash::{Table, simple_hash, probe};

        impl<'a> Table<&'a str> for [Option<Opcode>] {
            fn len(&self) -> usize {
                self.len()
            }

            fn key(&self, idx: usize) -> Option<&'a str> {
                self[idx].map(opcode_name)
            }
        }

        match probe::<&str, [Option<Opcode>]>(&OPCODE_HASH_TABLE, s, simple_hash(s)) {
            None => Err("Unknown opcode"),
            // We unwrap here because probe() should have ensured that the entry
            // at this index is not None.
            Some(i) => Ok(OPCODE_HASH_TABLE[i].unwrap()),
        }
    }
}

/// Contents on an instruction.
///
/// Every variant must contain `opcode` and `ty` fields. An instruction that doesn't produce a
/// value should have its `ty` field set to `VOID`. The size of `InstructionData` should be kept at
/// 16 bytes on 64-bit architectures. If more space is needed to represent an instruction, use a
/// `Box<AuxData>` to store the additional information out of line.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum InstructionData {
    Nullary { opcode: Opcode },
    Unary { opcode: Opcode, arg: Value },
    UnaryImm { opcode: Opcode, imm: Imm64 },
    UnaryIeee32 { opcode: Opcode, imm: Ieee32 },
    UnaryIeee64 { opcode: Opcode, imm: Ieee64 },
    Binary { opcode: Opcode, args: [Value; 2] },
    BinaryImm {
        opcode: Opcode,
        arg: Value,
        imm: Imm64,
    },
    Ternary { opcode: Opcode, args: [Value; 3] },
    MultiAry { opcode: Opcode, args: ValueList },
    InsertLane {
        opcode: Opcode,
        lane: Uimm8,
        args: [Value; 2],
    },
    ExtractLane {
        opcode: Opcode,
        lane: Uimm8,
        arg: Value,
    },
    IntCompare {
        opcode: Opcode,
        cond: IntCC,
        args: [Value; 2],
    },
    IntCompareImm {
        opcode: Opcode,
        cond: IntCC,
        arg: Value,
        imm: Imm64,
    },
    FloatCompare {
        opcode: Opcode,
        cond: FloatCC,
        args: [Value; 2],
    },
    Jump {
        opcode: Opcode,
        destination: Ebb,
        args: ValueList,
    },
    Branch {
        opcode: Opcode,
        destination: Ebb,
        args: ValueList,
    },
    BranchIcmp {
        opcode: Opcode,
        cond: IntCC,
        destination: Ebb,
        args: ValueList,
    },
    BranchTable {
        opcode: Opcode,
        arg: Value,
        table: JumpTable,
    },
    Call {
        opcode: Opcode,
        func_ref: FuncRef,
        args: ValueList,
    },
    IndirectCall {
        opcode: Opcode,
        sig_ref: SigRef,
        args: ValueList,
    },
    StackLoad {
        opcode: Opcode,
        stack_slot: StackSlot,
        offset: Offset32,
    },
    StackStore {
        opcode: Opcode,
        arg: Value,
        stack_slot: StackSlot,
        offset: Offset32,
    },
    HeapLoad {
        opcode: Opcode,
        arg: Value,
        offset: Uoffset32,
    },
    HeapStore {
        opcode: Opcode,
        args: [Value; 2],
        offset: Uoffset32,
    },
    Load {
        opcode: Opcode,
        flags: MemFlags,
        arg: Value,
        offset: Offset32,
    },
    Store {
        opcode: Opcode,
        flags: MemFlags,
        args: [Value; 2],
        offset: Offset32,
    },
    RegMove {
        opcode: Opcode,
        arg: Value,
        src: RegUnit,
        dst: RegUnit,
    },
}

/// A variable list of `Value` operands used for function call arguments and passing arguments to
/// basic blocks.
#[derive(Clone, Debug)]
pub struct VariableArgs(Vec<Value>);

impl VariableArgs {
    /// Create an empty argument list.
    pub fn new() -> VariableArgs {
        VariableArgs(Vec::new())
    }

    /// Add an argument to the end.
    pub fn push(&mut self, v: Value) {
        self.0.push(v)
    }

    /// Check if the list is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Convert this to a value list in `pool` with `fixed` prepended.
    pub fn into_value_list(self, fixed: &[Value], pool: &mut ValueListPool) -> ValueList {
        let mut vlist = ValueList::default();
        vlist.extend(fixed.iter().cloned(), pool);
        vlist.extend(self.0, pool);
        vlist
    }
}

// Coerce `VariableArgs` into a `&[Value]` slice.
impl Deref for VariableArgs {
    type Target = [Value];

    fn deref<'a>(&'a self) -> &'a [Value] {
        &self.0
    }
}

impl DerefMut for VariableArgs {
    fn deref_mut<'a>(&'a mut self) -> &'a mut [Value] {
        &mut self.0
    }
}

impl Display for VariableArgs {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        for (i, val) in self.0.iter().enumerate() {
            if i == 0 {
                write!(fmt, "{}", val)?;
            } else {
                write!(fmt, ", {}", val)?;
            }
        }
        Ok(())
    }
}

impl Default for VariableArgs {
    fn default() -> VariableArgs {
        VariableArgs::new()
    }
}

/// Analyzing an instruction.
///
/// Avoid large matches on instruction formats by using the methods defined here to examine
/// instructions.
impl InstructionData {
    /// Return information about the destination of a branch or jump instruction.
    ///
    /// Any instruction that can transfer control to another EBB reveals its possible destinations
    /// here.
    pub fn analyze_branch<'a>(&'a self, pool: &'a ValueListPool) -> BranchInfo<'a> {
        match *self {
            InstructionData::Jump {
                destination,
                ref args,
                ..
            } => BranchInfo::SingleDest(destination, args.as_slice(pool)),
            InstructionData::Branch {
                destination,
                ref args,
                ..
            } => BranchInfo::SingleDest(destination, &args.as_slice(pool)[1..]),
            InstructionData::BranchIcmp {
                destination,
                ref args,
                ..
            } => BranchInfo::SingleDest(destination, &args.as_slice(pool)[2..]),
            InstructionData::BranchTable { table, .. } => BranchInfo::Table(table),
            _ => BranchInfo::NotABranch,
        }
    }

    /// Get the single destination of this branch instruction, if it is a single destination
    /// branch or jump.
    ///
    /// Multi-destination branches like `br_table` return `None`.
    pub fn branch_destination(&self) -> Option<Ebb> {
        match *self {
            InstructionData::Jump { destination, .. } => Some(destination),
            InstructionData::Branch { destination, .. } => Some(destination),
            InstructionData::BranchIcmp { destination, .. } => Some(destination),
            _ => None,
        }
    }

    /// Get a mutable reference to the single destination of this branch instruction, if it is a
    /// single destination branch or jump.
    ///
    /// Multi-destination branches like `br_table` return `None`.
    pub fn branch_destination_mut(&mut self) -> Option<&mut Ebb> {
        match *self {
            InstructionData::Jump { ref mut destination, .. } => Some(destination),
            InstructionData::Branch { ref mut destination, .. } => Some(destination),
            InstructionData::BranchIcmp { ref mut destination, .. } => Some(destination),
            _ => None,
        }
    }

    /// Return information about a call instruction.
    ///
    /// Any instruction that can call another function reveals its call signature here.
    pub fn analyze_call<'a>(&'a self, pool: &'a ValueListPool) -> CallInfo<'a> {
        match *self {
            InstructionData::Call { func_ref, ref args, .. } => {
                CallInfo::Direct(func_ref, args.as_slice(pool))
            }
            InstructionData::IndirectCall { sig_ref, ref args, .. } => {
                CallInfo::Indirect(sig_ref, &args.as_slice(pool)[1..])
            }
            _ => CallInfo::NotACall,
        }
    }
}

/// Information about branch and jump instructions.
pub enum BranchInfo<'a> {
    /// This is not a branch or jump instruction.
    /// This instruction will not transfer control to another EBB in the function, but it may still
    /// affect control flow by returning or trapping.
    NotABranch,

    /// This is a branch or jump to a single destination EBB, possibly taking value arguments.
    SingleDest(Ebb, &'a [Value]),

    /// This is a jump table branch which can have many destination EBBs.
    Table(JumpTable),
}

/// Information about call instructions.
pub enum CallInfo<'a> {
    /// This is not a call instruction.
    NotACall,

    /// This is a direct call to an external function declared in the preamble. See
    /// `DataFlowGraph.ext_funcs`.
    Direct(FuncRef, &'a [Value]),

    /// This is an indirect call with the specified signature. See `DataFlowGraph.signatures`.
    Indirect(SigRef, &'a [Value]),
}

/// Value type constraints for a given opcode.
///
/// The `InstructionFormat` determines the constraints on most operands, but `Value` operands and
/// results are not determined by the format. Every `Opcode` has an associated
/// `OpcodeConstraints` object that provides the missing details.
#[derive(Clone, Copy)]
pub struct OpcodeConstraints {
    /// Flags for this opcode encoded as a bit field:
    ///
    /// Bits 0-2:
    ///     Number of fixed result values. This does not include `variable_args` results as are
    ///     produced by call instructions.
    ///
    /// Bit 3:
    ///     This opcode is polymorphic and the controlling type variable can be inferred from the
    ///     designated input operand. This is the `typevar_operand` index given to the
    ///     `InstructionFormat` meta language object. When this bit is not set, the controlling
    ///     type variable must be the first output value instead.
    ///
    /// Bit 4:
    ///     This opcode is polymorphic and the controlling type variable does *not* appear as the
    ///     first result type.
    ///
    /// Bits 5-7:
    ///     Number of fixed value arguments. The minimum required number of value operands.
    flags: u8,

    /// Permitted set of types for the controlling type variable as an index into `TYPE_SETS`.
    typeset_offset: u8,

    /// Offset into `OPERAND_CONSTRAINT` table of the descriptors for this opcode. The first
    /// `fixed_results()` entries describe the result constraints, then follows constraints for the
    /// fixed `Value` input operands. (`fixed_value_arguments()` of them).
    constraint_offset: u16,
}

impl OpcodeConstraints {
    /// Can the controlling type variable for this opcode be inferred from the designated value
    /// input operand?
    /// This also implies that this opcode is polymorphic.
    pub fn use_typevar_operand(self) -> bool {
        (self.flags & 0x8) != 0
    }

    /// Is it necessary to look at the designated value input operand in order to determine the
    /// controlling type variable, or is it good enough to use the first return type?
    ///
    /// Most polymorphic instructions produce a single result with the type of the controlling type
    /// variable. A few polymorphic instructions either don't produce any results, or produce
    /// results with a fixed type. These instructions return `true`.
    pub fn requires_typevar_operand(self) -> bool {
        (self.flags & 0x10) != 0
    }

    /// Get the number of *fixed* result values produced by this opcode.
    /// This does not include `variable_args` produced by calls.
    pub fn fixed_results(self) -> usize {
        (self.flags & 0x7) as usize
    }

    /// Get the number of *fixed* input values required by this opcode.
    ///
    /// This does not include `variable_args` arguments on call and branch instructions.
    ///
    /// The number of fixed input values is usually implied by the instruction format, but
    /// instruction formats that use a `ValueList` put both fixed and variable arguments in the
    /// list. This method returns the *minimum* number of values required in the value list.
    pub fn fixed_value_arguments(self) -> usize {
        ((self.flags >> 5) & 0x7) as usize
    }

    /// Get the offset into `TYPE_SETS` for the controlling type variable.
    /// Returns `None` if the instruction is not polymorphic.
    fn typeset_offset(self) -> Option<usize> {
        let offset = self.typeset_offset as usize;
        if offset < TYPE_SETS.len() {
            Some(offset)
        } else {
            None
        }
    }

    /// Get the offset into OPERAND_CONSTRAINTS where the descriptors for this opcode begin.
    fn constraint_offset(self) -> usize {
        self.constraint_offset as usize
    }

    /// Get the value type of result number `n`, having resolved the controlling type variable to
    /// `ctrl_type`.
    pub fn result_type(self, n: usize, ctrl_type: Type) -> Type {
        assert!(n < self.fixed_results(), "Invalid result index");
        if let ResolvedConstraint::Bound(t) =
            OPERAND_CONSTRAINTS[self.constraint_offset() + n].resolve(ctrl_type) {
            t
        } else {
            panic!("Result constraints can't be free");
        }
    }

    /// Get the value type of input value number `n`, having resolved the controlling type variable
    /// to `ctrl_type`.
    ///
    /// Unlike results, it is possible for some input values to vary freely within a specific
    /// `ValueTypeSet`. This is represented with the `ArgumentConstraint::Free` variant.
    pub fn value_argument_constraint(self, n: usize, ctrl_type: Type) -> ResolvedConstraint {
        assert!(n < self.fixed_value_arguments(),
                "Invalid value argument index");
        let offset = self.constraint_offset() + self.fixed_results();
        OPERAND_CONSTRAINTS[offset + n].resolve(ctrl_type)
    }

    /// Get the typeset of allowed types for the controlling type variable in a polymorphic
    /// instruction.
    pub fn ctrl_typeset(self) -> Option<ValueTypeSet> {
        self.typeset_offset().map(|offset| TYPE_SETS[offset])
    }

    /// Is this instruction polymorphic?
    pub fn is_polymorphic(self) -> bool {
        self.ctrl_typeset().is_some()
    }
}

type BitSet8 = BitSet<u8>;
type BitSet16 = BitSet<u16>;

/// A value type set describes the permitted set of types for a type variable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValueTypeSet {
    lanes: BitSet16,
    ints: BitSet8,
    floats: BitSet8,
    bools: BitSet8,
}

impl ValueTypeSet {
    /// Is `scalar` part of the base type set?
    ///
    /// Note that the base type set does not have to be included in the type set proper.
    fn is_base_type(&self, scalar: Type) -> bool {
        let l2b = scalar.log2_lane_bits();
        if scalar.is_int() {
            self.ints.contains(l2b)
        } else if scalar.is_float() {
            self.floats.contains(l2b)
        } else if scalar.is_bool() {
            self.bools.contains(l2b)
        } else {
            false
        }
    }

    /// Does `typ` belong to this set?
    pub fn contains(&self, typ: Type) -> bool {
        let l2l = typ.log2_lane_count();
        self.lanes.contains(l2l) && self.is_base_type(typ.lane_type())
    }

    /// Get an example member of this type set.
    ///
    /// This is used for error messages to avoid suggesting invalid types.
    pub fn example(&self) -> Type {
        let t = if self.ints.max().unwrap_or(0) > 5 {
            types::I32
        } else if self.floats.max().unwrap_or(0) > 5 {
            types::F32
        } else if self.bools.max().unwrap_or(0) > 5 {
            types::B32
        } else {
            types::B1
        };
        t.by(1 << self.lanes.min().unwrap()).unwrap()
    }
}

/// Operand constraints. This describes the value type constraints on a single `Value` operand.
enum OperandConstraint {
    /// This operand has a concrete value type.
    Concrete(Type),

    /// This operand can vary freely within the given type set.
    /// The type set is identified by its index into the TYPE_SETS constant table.
    Free(u8),

    /// This operand is the same type as the controlling type variable.
    Same,

    /// This operand is `ctrlType.lane_type()`.
    LaneOf,

    /// This operand is `ctrlType.as_bool()`.
    AsBool,

    /// This operand is `ctrlType.half_width()`.
    HalfWidth,

    /// This operand is `ctrlType.double_width()`.
    DoubleWidth,

    /// This operand is `ctrlType.half_vector()`.
    HalfVector,

    /// This operand is `ctrlType.double_vector()`.
    DoubleVector,
}

impl OperandConstraint {
    /// Resolve this operand constraint into a concrete value type, given the value of the
    /// controlling type variable.
    pub fn resolve(&self, ctrl_type: Type) -> ResolvedConstraint {
        use self::OperandConstraint::*;
        use self::ResolvedConstraint::Bound;
        match *self {
            Concrete(t) => Bound(t),
            Free(vts) => ResolvedConstraint::Free(TYPE_SETS[vts as usize]),
            Same => Bound(ctrl_type),
            LaneOf => Bound(ctrl_type.lane_type()),
            AsBool => Bound(ctrl_type.as_bool()),
            HalfWidth => Bound(ctrl_type.half_width().expect("invalid type for half_width")),
            DoubleWidth => {
                Bound(ctrl_type
                          .double_width()
                          .expect("invalid type for double_width"))
            }
            HalfVector => {
                Bound(ctrl_type
                          .half_vector()
                          .expect("invalid type for half_vector"))
            }
            DoubleVector => Bound(ctrl_type.by(2).expect("invalid type for double_vector")),
        }
    }
}

/// The type constraint on a value argument once the controlling type variable is known.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ResolvedConstraint {
    /// The operand is bound to a known type.
    Bound(Type),
    /// The operand type can vary freely within the given set.
    Free(ValueTypeSet),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opcodes() {
        use std::mem;

        let x = Opcode::Iadd;
        let mut y = Opcode::Isub;

        assert!(x != y);
        y = Opcode::Iadd;
        assert_eq!(x, y);
        assert_eq!(x.format(), InstructionFormat::Binary);

        assert_eq!(format!("{:?}", Opcode::IaddImm), "IaddImm");
        assert_eq!(Opcode::IaddImm.to_string(), "iadd_imm");

        // Check the matcher.
        assert_eq!("iadd".parse::<Opcode>(), Ok(Opcode::Iadd));
        assert_eq!("iadd_imm".parse::<Opcode>(), Ok(Opcode::IaddImm));
        assert_eq!("iadd\0".parse::<Opcode>(), Err("Unknown opcode"));
        assert_eq!("".parse::<Opcode>(), Err("Unknown opcode"));
        assert_eq!("\0".parse::<Opcode>(), Err("Unknown opcode"));

        // Opcode is a single byte, and because Option<Opcode> originally came to 2 bytes, early on
        // Opcode included a variant NotAnOpcode to avoid the unnecessary bloat. Since then the Rust
        // compiler has brought in NonZero optimization, meaning that an enum not using the 0 value
        // can be optional for no size cost. We want to ensure Option<Opcode> remains small.
        assert_eq!(mem::size_of::<Opcode>(), mem::size_of::<Option<Opcode>>());
    }

    #[test]
    fn instruction_data() {
        use std::mem;
        // The size of the `InstructionData` enum is important for performance. It should not
        // exceed 16 bytes. Use `Box<FooData>` out-of-line payloads for instruction formats that
        // require more space than that. It would be fine with a data structure smaller than 16
        // bytes, but what are the odds of that?
        assert_eq!(mem::size_of::<InstructionData>(), 16);
    }

    #[test]
    fn constraints() {
        let a = Opcode::Iadd.constraints();
        assert!(a.use_typevar_operand());
        assert!(!a.requires_typevar_operand());
        assert_eq!(a.fixed_results(), 1);
        assert_eq!(a.fixed_value_arguments(), 2);
        assert_eq!(a.result_type(0, types::I32), types::I32);
        assert_eq!(a.result_type(0, types::I8), types::I8);
        assert_eq!(a.value_argument_constraint(0, types::I32),
                   ResolvedConstraint::Bound(types::I32));
        assert_eq!(a.value_argument_constraint(1, types::I32),
                   ResolvedConstraint::Bound(types::I32));

        let b = Opcode::Bitcast.constraints();
        assert!(!b.use_typevar_operand());
        assert!(!b.requires_typevar_operand());
        assert_eq!(b.fixed_results(), 1);
        assert_eq!(b.fixed_value_arguments(), 1);
        assert_eq!(b.result_type(0, types::I32), types::I32);
        assert_eq!(b.result_type(0, types::I8), types::I8);
        match b.value_argument_constraint(0, types::I32) {
            ResolvedConstraint::Free(vts) => assert!(vts.contains(types::F32)),
            _ => panic!("Unexpected constraint from value_argument_constraint"),
        }

        let c = Opcode::Call.constraints();
        assert_eq!(c.fixed_results(), 0);
        assert_eq!(c.fixed_value_arguments(), 0);

        let i = Opcode::CallIndirect.constraints();
        assert_eq!(i.fixed_results(), 0);
        assert_eq!(i.fixed_value_arguments(), 1);

        let cmp = Opcode::Icmp.constraints();
        assert!(cmp.use_typevar_operand());
        assert!(cmp.requires_typevar_operand());
        assert_eq!(cmp.fixed_results(), 1);
        assert_eq!(cmp.fixed_value_arguments(), 2);
    }

    #[test]
    fn value_set() {
        use ir::types::*;

        let vts = ValueTypeSet {
            lanes: BitSet16::from_range(0, 8),
            ints: BitSet8::from_range(4, 7),
            floats: BitSet8::from_range(0, 0),
            bools: BitSet8::from_range(3, 7),
        };
        assert!(!vts.contains(I8));
        assert!(vts.contains(I32));
        assert!(vts.contains(I64));
        assert!(vts.contains(I32X4));
        assert!(!vts.contains(F32));
        assert!(!vts.contains(B1));
        assert!(vts.contains(B8));
        assert!(vts.contains(B64));
        assert_eq!(vts.example().to_string(), "i32");

        let vts = ValueTypeSet {
            lanes: BitSet16::from_range(0, 8),
            ints: BitSet8::from_range(0, 0),
            floats: BitSet8::from_range(5, 7),
            bools: BitSet8::from_range(3, 7),
        };
        assert_eq!(vts.example().to_string(), "f32");

        let vts = ValueTypeSet {
            lanes: BitSet16::from_range(1, 8),
            ints: BitSet8::from_range(0, 0),
            floats: BitSet8::from_range(5, 7),
            bools: BitSet8::from_range(3, 7),
        };
        assert_eq!(vts.example().to_string(), "f32x2");

        let vts = ValueTypeSet {
            lanes: BitSet16::from_range(2, 8),
            ints: BitSet8::from_range(0, 0),
            floats: BitSet8::from_range(0, 0),
            bools: BitSet8::from_range(3, 7),
        };
        assert!(!vts.contains(B32X2));
        assert!(vts.contains(B32X4));
        assert_eq!(vts.example().to_string(), "b32x4");

        let vts = ValueTypeSet {
            // TypeSet(lanes=(1, 256), ints=(8, 64))
            lanes: BitSet16::from_range(0, 9),
            ints: BitSet8::from_range(3, 7),
            floats: BitSet8::from_range(0, 0),
            bools: BitSet8::from_range(0, 0),
        };
        assert!(vts.contains(I32));
        assert!(vts.contains(I32X4));
    }
}
