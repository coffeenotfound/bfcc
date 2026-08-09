#![feature(exit_status_error)]

mod anal;

use std::{env, fmt, fs};
use std::ffi::OsStr;
use std::fmt::{Formatter, Write};
use std::process::{Command, Stdio};
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
	let out_buf_len = 32;
	let mut out_buf_offset = 0;
	
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
	
	// Read c boilerplate
	let mut whole_c = fs::read_to_string("boilerplate.c")?;
	whole_c = whole_c.replace("/*OUT_BUF_LEN*/", &format!("{out_buf_len}"));
	
	let mut c = String::new();
	
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
				_ = writeln!(c, "c[{}] += {};", frame.curr_offset, num & 0xff);
			}
			Token::Decr(num) => {
				_ = writeln!(c, "c[{}] -= {};", frame.curr_offset, num & 0xff);
			}
			Token::Out(num) => {
//				for _ in 0..*num {
//					_ = writeln!(c, "{ind}putchar(*(c{}));", print_offset(frame.curr_offset));
//				}
				_ = writeln!(c, "o[{}] = c[{}];", out_buf_offset, frame.curr_offset);
				out_buf_offset += 1;
				
				if out_buf_offset >= out_buf_len {
					_ = writeln!(c, "print_now({out_buf_offset});");
					out_buf_offset = 0;
				}
			}
			Token::In(num) => {
				for _ in 0..*num {
					_ = writeln!(c, "c[{}] = getchar();", frame.curr_offset);
				}
			}
			Token::LoopStart => {
				// Materialize the frame offset before
				// the new loop
				// TODO: If the loop is head-neutral, I think
				//  we could skip this, but we don't analyze for that yet
				_ = writeln!(c, "c = c{};", print_offset(frame.curr_offset));
				frame.curr_offset = 0; // reset
				
				// Materialize out buffer
				if out_buf_offset > 0 {
					_ = writeln!(c, "print_now({out_buf_offset});");
					out_buf_offset = 0;
				}
				
				// Push new frame
				frames.push(Frame {
					curr_offset: 0,
				});
				
				// Start while loop
				_ = writeln!(c, "while (*c) {{");
			}
			Token::LoopEnd => {
				// Materialize the frame offset before
				// the loop end
				// TODO: If the loop is head-neutral, I think
				//  we could skip this, but we don't analyze for that yet
				_ = writeln!(c, "c = c{};", print_offset(frame.curr_offset));
				
				// Materialize out buffer
				if out_buf_offset > 0 {
					_ = writeln!(c, "print_now({out_buf_offset});");
					out_buf_offset = 0;
				}
				
				// Pop frame
				frames.pop();
				
				// End while loop
				_ = writeln!(c, "}}");
			}
		}
		
		tok_idx += 1;
	}
	
	// Materialize out buffer
	if out_buf_offset > 0 {
		_ = writeln!(c, "print_now({out_buf_offset});");
	}
	
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
			"-o", "out",
			"out.c",
		],
	)?;
	
	// Run compiled program
	println!("--- running compiled binary ---");
	Command::new(env::current_dir()?.join("out"))
		.spawn()?
		.wait()?;
	
	Ok(())
}

fn exec_process<I, S>(
	binary: &str,
	args: I,
) -> Result<(), anyhow::Error> where 
	I: IntoIterator<Item = S>,
    S: AsRef<OsStr>
{
	let exit_status = Command::new(binary)
		.args(args)
		.stdout(Stdio::inherit())
		.stderr(Stdio::inherit())
		.spawn()?
		.wait()?;
	
	if !exit_status.success() {
		bail!("process exited with non-zero status: {exit_status}");
	}
	
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
