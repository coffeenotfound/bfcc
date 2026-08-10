use std::fmt::Write;
use std::collections::BTreeMap;
use std::{env, fs};
use std::process::{Command, Output};
use crate::exec_process;
use crate::parse::{parse, remove_frontmatter_tokens, Token};

pub fn main_opt() -> Result<(), anyhow::Error> {
	let code = include_bytes!("../snippets/hello_world.bf");
//	let code = include_bytes!("../snippets/golden.bf");
//	let code = b"[hello]+++[>++>+++<<-]";
	
	// Lex code
	let toks = parse(code)?;
	println!("{toks:?}");
	
	// Construct initial ir
	let (ir_root, _) = construct_ir(&toks, &mut 0);
	println!("{ir_root:#?}");
	
	// Optimize code
	let mut opt_ir = ir_root.clone();
	opt_remove_frontmatter(&mut opt_ir);
	opt_mul_loops(&mut opt_ir);
	
	{// Compile to c
		let mut whole_c = fs::read_to_string("boilerplate.c")?;
		whole_c = whole_c.replace("/*OUT_BUF_LEN*/", &format!("{OUT_BUF_SIZE}"));
		
		let mut c = String::new();
		write_ir_body_to_c(&mut c, &opt_ir);
		
		whole_c = whole_c.replace("/*MAIN_CODE*/", &c);
		
		fs::write("out.c", &whole_c)?;
		
		// Clang-format generated c
		exec_process(
			"clang-format",
			[
				"-style={BasedOnStyle: WebKit, UseTab: Always, IndentWidth: 4, TabWidth: 4}",
				"-i", "out.c",
			]
		)?;
		
		// Compile generated c
		exec_process(
			"clang",
			[
				"-O3",
				"-march=znver2",  // good baseline
				"-fwrapv",  // wrapping overflow is bf semantics
				"-o", "out",
				"out.c",
			],
		)?;
		
		// Run compiled program
		println!("--- running compiled binary ---");
		Command::new(env::current_dir()?.join("out"))
			.spawn()?
			.wait()?;
	}
	
	std::process::exit(0);
}

/// In future this would be done intrinsically
/// by sparse const prop
pub fn opt_remove_frontmatter(
	root: &mut Vec<IrOp>,
) {
	let mut new_ops = vec![];
	let mut emit_everything_now = false;
	
	for (idx, op) in root.iter().enumerate() {
		if op.writes_cells_by_itself() {
			emit_everything_now = true;
		}
		
		if !matches!(op, IrOp::Loop(..)) || emit_everything_now {
			new_ops.push(op.clone());
		}
	}
	
	*root = new_ops;
}

pub fn opt_mul_loops(
	block: &mut Vec<IrOp>,
) -> bool {
	let mut did_something = false;
	
	// TODO: giga-inefficient
	let mut new_block = vec![];
	let mut offset_to_incr_delta = BTreeMap::<i32, i32>::new();
	
	for op in block.iter_mut() {
		let mut replaced_op = false;
		
		if let IrOp::Loop(loop_) = op {
			let mut definitely_not_a_mul_loop = false;
			offset_to_incr_delta.clear();
			
			for loop_op in loop_.body.iter() {
				if let IrOp::Incr { val, offset } = loop_op{
					*offset_to_incr_delta.entry(*offset).or_insert(0) += *val;
				} else {
					definitely_not_a_mul_loop = true;
					break;
				}
			}
			
			// analyze sub loops
			did_something |= opt_mul_loops(&mut loop_.body);
			
			if !definitely_not_a_mul_loop {
				// We can only opt mul loop with induction delta = -1 for now
				if let Some(&induction_delta) = offset_to_incr_delta.get(&0)
					&& induction_delta == -1
				{
					for (&offset, &incr_delta) in &offset_to_incr_delta {
						if offset != 0 {
							new_block.push(IrOp::MulBy {
								src_offset: loop_.move_head_prior,
								mul: incr_delta,
								dst_offset: loop_.move_head_prior + offset,
							});
						}
					}
					
					new_block.push(IrOp::Set {
						offset: loop_.move_head_prior,
						val: 0,
					});
					
					replaced_op = true;
				}
			}
		}
		
		if replaced_op {
			did_something = true;
		} else {
			new_block.push(op.clone());
		}
	}
	
	*block = new_block;
	
	did_something
}

const OUT_BUF_SIZE: usize = 32;

pub fn write_ir_body_to_c(
	c: &mut String,
	ir: &[IrOp],
) {
	let mut out_buf_offset = 0;
	
	for op in ir {
		match op {
			IrOp::Incr { val, offset } => {
				let sign = if val.is_positive() { "+" } else { "-" };
				_ = writeln!(c, "c[{}] {}= {};", offset, sign, val.abs());
			}
			IrOp::Set { val, offset } => {
				_ = writeln!(c, "c[{}] = {};", offset, *val as u8);
			}
			IrOp::MulBy { src_offset, mul, dst_offset } => {
				_ = writeln!(c, "c[{}] = c[{}] * {};", dst_offset, src_offset, mul);
			}
			IrOp::Input { count, offset } => {
				for _ in 0..*count {
					_ = writeln!(c, "c[{}] = getchar();", offset);
				}
				
			}
			IrOp::Output { count, offset } => {
				_ = writeln!(c, "o[{}] = c[{}];", out_buf_offset, offset);
				out_buf_offset += 1;
				
				if out_buf_offset >= OUT_BUF_SIZE {
					_ = writeln!(c, "print_now(o, {out_buf_offset});");
					out_buf_offset = 0;
				}
			}
			IrOp::Loop(lp) => {
				// Materialize out buffer before loop
				if out_buf_offset > 0 {
					_ = writeln!(c, "print_now(o, {out_buf_offset});");
					out_buf_offset = 0;
				}
				
				// Move head before loop
				if lp.move_head_prior > 0 {
					_ = writeln!(c, "c += {};", lp.move_head_prior);
				} else if lp.move_head_prior < 0 {
					_ = writeln!(c, "c -= {};", lp.move_head_prior.abs());
				}
				
				// Emit while loop
				_ = writeln!(c, "while (*c) {{");
				write_ir_body_to_c(c, &lp.body);
				
				// Move head after iter
				if lp.move_head_after_iter > 0 {
					_ = writeln!(c, "c += {};", lp.move_head_after_iter);
				} else if lp.move_head_after_iter < 0 {
					_ = writeln!(c, "c -= {};", lp.move_head_after_iter.abs());
				}
				
				_ = writeln!(c, "}}");
			}
		}
	}
	
	// Materialize out buffer before end
	if out_buf_offset > 0 {
		_ = writeln!(c, "print_now(o, {out_buf_offset});");
	}
}

pub fn construct_ir(
	toks: &[Token],
	tok_idx: &mut usize,
) -> (Vec<IrOp>, i32) {
	let mut head_offset = 0;
//	let mut had_scanning_loop = false;
	let mut ops = vec![];
	
	while !(*tok_idx >= toks.len()) {
		let tok = &toks[*tok_idx];
		match tok {
			Token::Left(num) => {
				head_offset -= *num as i32;
			}
			Token::Right(num) => {
				head_offset += *num as i32;
			}
			Token::Incr(num) => {
				ops.push(IrOp::Incr {
					val: *num as i32,
					offset: head_offset,
				});
			}
			Token::Decr(num) => {
				ops.push(IrOp::Incr {
					val: -(*num as i32),
					offset: head_offset,
				});
			}
			Token::Out(num) => {
				ops.push(IrOp::Output {
					count: *num,
					offset: head_offset,
				});
			}
			Token::In(num) => {
				ops.push(IrOp::Input {
					count: *num,
					offset: head_offset,
				});
			}
			Token::LoopStart => {
				// Construct loop
				*tok_idx += 1;
				let (body, inner_loop_delta) = construct_ir(toks, tok_idx);
				
				ops.push(IrOp::Loop(IrLoop {
					move_head_prior: head_offset,
					move_head_after_iter: inner_loop_delta,
					body,
				}));
				head_offset = 0;
			}
			Token::LoopEnd => {
//				*tok_idx += 1;
				break;
			}
		}
		
		*tok_idx += 1;
	}
	
	(ops, head_offset)
}

#[derive(Clone, Debug)]
pub enum IrOp {
	Incr {
		val: i32,
		offset: i32,
	},
	Set {
		val: i32,
		offset: i32,
	},
	MulBy {
		src_offset: i32,
		/// TODO: For now we only support loops with induction_delta = -1,
		///  since those are the easiest
//		/// By how much the induction cell is changed
//		/// each iter
//		/// 
//		/// Needed for e.g. `[>+< ---]` (induction decr by 3) where we need
//		/// to figure out how many iters until we *actually* hit exactly 0
//		induction_delta: i32,
		mul: i32,
		dst_offset: i32,
	},
	Input {
		count: u32,
		offset: i32,
	},
	Output {
		count: u32,
		offset: i32,
	},
//	MoveHead {
//		delta: i32,
//	},
	Loop(IrLoop),
}

impl IrOp {
	/// Used for frontmatter removal
	/// 
	/// Loop body is not counted, loops always return false
	pub fn writes_cells_by_itself(&self) -> bool {
		match self {
			IrOp::Incr { .. }
			| IrOp::Set { .. }
			| IrOp::MulBy { .. }
			| IrOp::Input { .. } => true,
			
			IrOp::Output { .. }
			| IrOp::Loop(..) => false,
		}
	}
}

#[derive(Clone, Debug)]
pub struct IrLoop {
//	/// Delta the head is moved by each iteration
//	/// 
//	/// For more complex nested scanning loops like `[ [<] > [>] <<<]`,
//	/// the outer loop has `None` here since we just don't know
//	pub iter_head_delta: Option<i32>,
	pub move_head_prior: i32,
	pub move_head_after_iter: i32,
	
	pub body: Vec<IrOp>,
}
