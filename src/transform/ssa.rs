//! Implements the SSA construction algorithm described in
//! "Simple and Efficient Construction of Static Single Assignment Form"

use std::collections::HashMap;
use petgraph::graph::NodeIndex;
use frontend::structs::LRegInfo;
use middle::cfg::NodeData as CFGNodeData;
use middle::cfg::EdgeType as CFGEdgeType;
use middle::cfg::{CFG, BasicBlock};
use middle::ssa::{BBInfo, SSA, SSAMod, ValueType};
use middle::ir::{MVal, MOpcode, MValType};
use middle::regfile::SubRegisterFile;
use transform::phiplacement::PhiPlacer;

pub type VarId = usize;

pub struct SSAConstruction<'a, T: SSAMod<BBInfo=BBInfo> + 'a> {
	pub phiplacer: PhiPlacer<'a, T>,
	pub regfile:   SubRegisterFile,
	pub temps:     HashMap<String, T::ValueRef>,
}

impl<'a, T: SSAMod<BBInfo=BBInfo> + 'a> SSAConstruction<'a, T> {
	pub fn new(ssa: &'a mut T, reg_info: &LRegInfo) -> SSAConstruction<'a, T> {
		let mut sc = SSAConstruction {
			phiplacer: PhiPlacer::new(ssa),
			regfile:   SubRegisterFile::new(reg_info),
			temps:     HashMap::new(),
		};
		// make the following a method of regfile?
		sc.phiplacer.add_variables(sc.regfile.whole_registers.clone());
		sc
	}

	pub fn run(&mut self, cfg: &CFG) {
		let mut blocks = Vec::<T::ActionRef>::new();
        let bb_iter = cfg.bbs.iter();

        {
            // Insert the entry and exit blocks for the ssa.
            let block = self.phiplacer.ssa.add_block(BBInfo { addr: 0 });
            self.phiplacer.ssa.mark_start_node(&block);
            self.incomplete_phis.insert(block, HashMap::new());
            blocks.push(block);

            let block = self.phiplacer.ssa.add_block(BBInfo { addr: 0 });
            self.incomplete_phis.insert(block, HashMap::new());
            blocks.push(block);
        }

        for (addr, i) in bb_iter {
            let block = self.phiplacer.ssa.add_block(BBInfo { addr: *addr });
            self.phiplacer.incomplete_phis.insert(block, HashMap::new());
            blocks.push(block);
            match cfg.g[*i] {
                CFGNodeData::Block(ref srcbb) => {
                    self.process_block(block, srcbb);
                }
                _ => unreachable!(),
            }
        }

		for edge in cfg.g.raw_edges() {
			let i = match edge.weight.edge_type {
				CFGEdgeType::False => 0,
				CFGEdgeType::True => 1,
				CFGEdgeType::Unconditional => 2,
			};
			self.phiplacer.ssa.add_control_edge(
				blocks[edge.source().index()],
				blocks[edge.target().index()],
				i);
		}
		for block in blocks {
			self.phiplacer.seal_block(block);
		}
		//self.phiplacer.ssa.stable_indexing = false;
		//self.phiplacer.ssa.cleanup();
	}

	fn process_in(&mut self, block: T::ActionRef, mval: &MVal) -> T::ValueRef {
		match mval.val_type {
			MValType::Register  => self.regfile.read_register(&mut self.phiplacer, 0, block, &mval.name),
			MValType::Temporary => self.temps[&mval.name],
			MValType::Unknown   => self.phiplacer.ssa.invalid_value(), //self.phiplacer.ssa.add_comment(block, &"Unknown".to_string()), // unimplemented!()
			MValType::Internal  => self.phiplacer.ssa.invalid_value(), //self.phiplacer.ssa.add_comment(block, &mval.name), // unimplemented!()
			MValType::Null      => self.phiplacer.ssa.invalid_value(),
		}
	}

	fn process_out(&mut self, block: T::ActionRef, mval: &MVal, value: T::ValueRef) {
		match mval.val_type {
			MValType::Register  => self.regfile.write_register(&mut self.phiplacer, 0, block, &mval.name, value),
			MValType::Temporary => {self.temps.insert(mval.name.clone(), value);},
			MValType::Unknown   => {}, // unimplemented!(),
			MValType::Internal  => {}, // unimplemented!()
			MValType::Null      => {},
		}
	}

	fn process_op(&mut self, block: T::ActionRef, optype: ValueType, opc: MOpcode, n0: T::ValueRef, n1: T::ValueRef) -> T::ValueRef {
		if opc == MOpcode::OpEq {
			return n0
		}
		let ref mut ssa = self.phiplacer.ssa;

        /*
        // TODO: When developing a ssa check pass, reuse this maybe
        let width = match opc {
            MOpcode::OpNarrow(w)
            | MOpcode::OpWiden(w) => { w },
            MOpcode::OpCmp => { 1 },
            _ => { 
                let extract = |x: NodeData| -> Option<u8> {
                    if let NodeData::Op(_, ValueType::Integer { width: w }) = x {
                        Some(w)
                    } else {
                        None
                    }
                };
                let w1 = self.phiplacer.ssa.safe_get_node_data(&n0)
                                 .map(&extract)
                                 .unwrap_or(None);

                let w2 = self.phiplacer.ssa.safe_get_node_data(&n1)
                                 .map(&extract)
                                 .unwrap_or(None);

                if w1 == None && w2 == None {
                    // TODO: Replace by default value.
                    64
                } else if w1 == None {
                    w2.unwrap()
                } else if w2 == None {
                    w1.unwrap()
                } else {
                    let w1 = w1.unwrap();
                    let w2 = w2.unwrap();
                    // Check the width of the two operands.
                    assert!(w1 == w2);
                    w1
                }
            },
        };*/

		let nn = ssa.add_op(block, opc, optype);
		ssa.op_use(nn, 0, n0);
		ssa.op_use(nn, 1, n1);
		return nn
	}

	fn process_block(&mut self, block: T::ActionRef, source: &BasicBlock) {
		for ref instruction in &source.instructions {
			let n0 = self.process_in(block, &instruction.operand_1);
			let n1 = self.process_in(block, &instruction.operand_2);

			if instruction.opcode == MOpcode::OpJmp {
				// TODO: In case of static jumps, this is trivial and does not need a selector.
                // In case of dynamic jump, the jump targets have to be determined.
				//self.ssa.g.add_edge(block, n0, SSAEdgeData::DynamicControl(0));
				break;
			}

			if instruction.opcode == MOpcode::OpCJmp {
				self.phiplacer.ssa.mark_selector(n0, block);
				continue;
			}

			let dsttype = match instruction.dst.val_type {
				MValType::Null => ValueType::Integer{width: 0}, // there is no ValueType::None?
				_              => ValueType::Integer{width: instruction.dst.size},
			};
			let nn = self.process_op(block, dsttype, instruction.opcode, n0, n1);
			self.process_out(block, &instruction.dst, nn);
		}
	}
}
