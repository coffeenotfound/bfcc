mod anal;

use std::{env, fmt, fs};
use std::fmt::{Formatter, Write};
use std::process::Command;
use anyhow::bail;
use crate::anal::main_anal;

struct FmtIndent(usize);
impl fmt::Display for FmtIndent {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		for _ in 0..self.0 {
			f.write_char('\t')?;
		}
		Ok(())
	}
}

fn main() -> Result<(), anyhow::Error> {
//	main_anal()?;
//	if 1 != 2 {
//		return Ok(());
//	}
	
	let code = include_bytes!("../hello_world.bf");
	
	// Lex code
	let toks = parse(code)?;
	
	// Remove frontmatter:
	// If all mem is still zero, the loop
	// cannot possibly execute any iters, so
	// don't even emit it (this is mostly for
	// removing beginning comment loops)
	let mut new_toks = vec![];
	let mut all_mem_still_zero = true;
	let mut tok_idx = 0;
	
	while !(tok_idx >= toks.len()) {
		let tok = &toks[tok_idx];
		
		match tok {
			Token::Incr(_)
			| Token::Decr(_)
			| Token::In(_) => {
				all_mem_still_zero = false;
			}
			_ => (),
		}
		
		// Skip full loop
		if let Token::LoopStart = tok && all_mem_still_zero {
			tok_idx += 1;
			
			let mut skip_loop_depth = 1;
			while !(tok_idx >= toks.len()) {
				let tok = &toks[tok_idx];
				
				match tok {
					Token::LoopStart => {
						skip_loop_depth += 1;
					}
					Token::LoopEnd => {
						skip_loop_depth -= 1;
					}
					_ => (),
				}
				
				if skip_loop_depth == 0 {
					break;
				}
				tok_idx += 1;
			}
		} else {
			new_toks.push(tok.clone());
		}
		
		tok_idx += 1;
	}
	
	let toks = new_toks;
	
	// Lower to C
	let mut c = String::new();
	
	let mut ind = FmtIndent(1);
	
	_ = writeln!(c, "#include <stdlib.h>");
	_ = writeln!(c, "#include <stdio.h>");
	_ = writeln!(c, "#include <string.h>");
	_ = writeln!(c, "");
	_ = writeln!(c, "#define NUM_CELLS (32*1024)");
	_ = writeln!(c, "");
	_ = writeln!(c, "int main() {{");
	_ = writeln!(c, "{ind}unsigned char *b = calloc(NUM_CELLS, sizeof(unsigned char));");
	_ = writeln!(c, "{ind}unsigned char *c = b;");
	_ = writeln!(c, "{ind}");
	
	struct Frame {
		curr_offset: i32,
	}
	let mut frames = vec![Frame {
		curr_offset: 0,
	}];
	
	let print_offset = |offset: i32| -> String {
		if offset > 0 {
			format!(" + {}", offset)
		} else if offset < 0 {
			format!(" - {}", offset.abs())
		} else {
			"".to_string()
		}
	};
	
	let mut tok_idx = 0;
	while !(tok_idx >= toks.len()) {
		let frame = frames.last_mut().unwrap();
		
		let tok = &toks[tok_idx];
		match tok {
			Token::Left(num) => {
				frame.curr_offset -= *num as i32;
//				_ = writeln!(c, "{ind}c -= {num}ull;");
			}
			Token::Right(num) => {
				frame.curr_offset += *num as i32;
//				_ = writeln!(c, "{ind}c += {num}ull;");
			}
			Token::Incr(num) => {
				_ = writeln!(c, "{ind}*(c{}) += {};", print_offset(frame.curr_offset), num & 0xff);
			}
			Token::Decr(num) => {
				_ = writeln!(c, "{ind}*(c{}) -= {};", print_offset(frame.curr_offset), num & 0xff);
			}
			Token::Out(num) => {
				for _ in 0..*num {
					_ = writeln!(c, "{ind}putchar(*(c{}));", print_offset(frame.curr_offset));
				}
			}
			Token::In(num) => {
				for _ in 0..*num {
					_ = writeln!(c, "{ind}*(c{}) = getchar();", print_offset(frame.curr_offset));
				}
			}
			Token::LoopStart => {
				// Materialize the frame offset before
				// the new loop
				// TODO: If the loop is head-neutral, I think
				//  we could skip this, but we don't analyze for that yet
				
				_ = writeln!(c, "{ind}c = c{};", print_offset(frame.curr_offset));
				frame.curr_offset = 0; // reset
				
				// Push new frame
				frames.push(Frame {
					curr_offset: 0,
				});
				
				// Start while loop
				_ = writeln!(c, "{ind}while (*c) {{");
				ind.0 += 1;
			}
			Token::LoopEnd => {
				// Materialize the frame offset before
				// the loop end
				// TODO: If the loop is head-neutral, I think
				//  we could skip this, but we don't analyze for that yet
				
				_ = writeln!(c, "{ind}c = c{};", print_offset(frame.curr_offset));
				
				// Pop frame
				frames.pop();
				
				// End while loop
				ind.0 -= 1;
				_ = writeln!(c, "{ind}}}");
			}
		}
		
		tok_idx += 1;
	}
	_ = writeln!(c, "");
	_ = writeln!(c, "{ind}free(b);");
	_ = writeln!(c, "{ind}return 0;");
	_ = writeln!(c, "}}");
	
	fs::write("out.c", c).unwrap();
	
	// Compile generated c
	Command::new("clang")
		.args([
			"-O3",
			"-o", "out",
			"out.c",
		])
		.spawn()?
		.wait()?;
	
	// Run compiled program
	println!("--- running compiled binary ---");
	Command::new(env::current_dir()?.join("out"))
		.spawn()?
		.wait()?;
	
	Ok(())
}

#[derive(Clone, Debug)] 
pub enum Token {
	Left(u64),
	Right(u64),
	Incr(u64),
	Decr(u64),
	Out(u64),
	In(u64),
	LoopStart,
	LoopEnd,
}

fn parse(text: &[u8]) -> Result<Vec<Token>, anyhow::Error> {
	macro_rules! op_with_num {
		($name:ident; $ir:expr) => {
			if let Some(Token::$name(num)) = $ir.last_mut()
				&& *num < u64::MAX
			{
				*num += 1;
			} else {
				$ir.push(Token::$name(1))
			}
		}
	}
	
	let mut ir = vec![];
	let mut loop_depth = 0_usize;
	
	for &b in text {
		match b {
			b'<' => op_with_num!(Left; ir),
			b'>' => op_with_num!(Right; ir),
			b'+' => op_with_num!(Incr; ir),
			b'-' => op_with_num!(Decr; ir),
			b'.' => op_with_num!(Out; ir),
			b',' => op_with_num!(In; ir),
			b'[' => {
				ir.push(Token::LoopStart);
				loop_depth += 1;
			},
			b']' => {
				ir.push(Token::LoopEnd);
				if loop_depth > 0 {
					loop_depth -= 1;
				} else {
					bail!("unmatched loop end");
				}
			},
			// ignore all other symbols
			_ => (),
		};
	}
	
	if loop_depth != 0 {
		bail!("unmatched loop start");
	}
	
	Ok(ir)
}
