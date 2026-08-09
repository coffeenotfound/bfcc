use std::cell::Cell;
use std::collections::BTreeMap;
use crate::{parse, Token};

pub fn main_anal() -> Result<(), anyhow::Error> {
	let code = include_bytes!("../snippet.bf");
	
	// Lex code
	let toks = parse(code)?;
	
	// NOTE: The tape does NOT wrap,
	//  over- and underflow are UB!
	
	let (ir, _) = convert_rec(
		&toks,
		&mut 0,
	)?;
	
	println!("{ir:#?}");
	
	Ok(())
}

#[derive(Clone, Debug)]
pub enum Ir {
	Shift {
		amount: i64,
	},
	Add {
//		offset: i64,
		/// subtract is just big add
		val: u8
	},
	Out {
//		offset: i64,
	},
	In {
//		offset: i64,
	},
	Loop {
//		/// Tape offset at which the loop starts it's
//		/// first iter (if any)
//		offset: i64,
//		/// How much the tape pointer shifts
//		/// from beginning to end of one loop iter
//		shift_per_iter: i64,
		body: Vec<Ir>,
	},
}

//#[derive(Copy, Clone, Debug)]
//pub enum LoopShift {
//	Fixed(i64),
//	DynLeft,
//	DynRight,
//}

#[derive(Clone, Debug)]
struct CellPreds {
	cells: BTreeMap<i64, CellPred>,
}

impl CellPreds {
	pub fn new() -> Self {
		Self {
			cells: BTreeMap::new()
		}
	}
	
	
}

#[derive(Copy, Clone, Debug)]
struct CellPred {
	pub known_val: Option<u8>,
}

impl CellPred {
	pub fn zero() -> Self {
		Self { known_val: Some(0) }
	}
}

fn anal_no_loops(
	ir: &[Ir],
) {
	let mut head = 0i64;
	
	let mut cell_states = BTreeMap::<i64, CellPred>::new();
	
	for op in ir {
		match op {
			Ir::Shift { amount } => {
				head += *amount;
			}
			Ir::Add { val } => {
				let mut pred = cell_states.entry(head)
					.or_insert(CellPred::zero());
				
				if let Some(known_val) = &mut pred.known_val {
					*known_val += *val;
				}
			}
			Ir::Out { .. } => {
				// has no side effects on our program state
				let mut pred = cell_states.get(&head);
				
//				if let Some(known_val) = &mut pred.known_val {
//					*known_val += *val;
//				}
			}
			Ir::In { .. } => {
				let mut pred = cell_states.entry(head)
					.or_insert(CellPred::zero());
				
				pred.known_val = None;
			}
			Ir::Loop { .. } => {
				// ignored for now
			}
		}
	}
}

fn convert_rec(
	toks: &[Token],
	tok_idx: &mut usize,
) -> Result<(Vec<Ir>, i64), anyhow::Error> {
	let mut ir = vec![];
	let mut curr_offset = 0i64;
	
	while !(*tok_idx >= toks.len()) {
		let tok = &toks[*tok_idx];
		match tok {
			Token::Left(num) => {
				curr_offset -= *num as i64;
				ir.push(Ir::Shift {
					amount: -(*num as i64),
				});
			}
			Token::Right(num) => {
				curr_offset += *num as i64;
				ir.push(Ir::Shift {
					amount: *num as i64,
				});
			}
			Token::Incr(num) => {
				ir.push(Ir::Add {
//					offset: curr_offset,
					val: *num as u8,
				});
			}
			Token::Decr(num) => {
				ir.push(Ir::Add {
//					offset: curr_offset,
					val: u8::wrapping_add(!(*num as u8), 1), // twos complement
				});
			}
			Token::Out(num) => {
				for _ in 0..*num {
					ir.push(Ir::Out {
//						offset: curr_offset,
					});
				}
			}
			Token::In(num) => {
				for _ in 0..*num {
					ir.push(Ir::In {
//						offset: curr_offset,
					});
				}
			}
			Token::LoopStart => {
//				ir.push(Ir::Shift {
//					amount: curr_offset,
//				});
//				curr_offset = 0;
				
				*tok_idx += 1;
				let (body, loop_shift) = convert_rec(
					toks,
					tok_idx,
				)?;
				
				ir.push(Ir::Loop {
//					offset: curr_offset,
//					shift_per_iter: loop_shift,
					body,
				});
			}
			Token::LoopEnd => {
				*tok_idx += 1;
				// We've reached the end of this loop, return
				return Ok((ir, curr_offset))
			}
		}
		
		*tok_idx += 1;
	}
	
	Ok((ir, curr_offset))
}
