pub mod parse;
pub mod opt;
pub mod patch_iter;

use crate::opt::main_opt;
use crate::parse::{parse, remove_frontmatter_tokens, Token};
use anyhow::bail;
use std::ffi::OsStr;
use std::fmt::{Formatter, Write};
use std::process::{Command, Stdio};
use std::{env, fmt, fs};

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
	// Run new backend instead
	main_opt()?;
	if 1 != 2 {
		return Ok(());
	}
	
	let code = include_bytes!("../snippets/hello_world.bf");
//	let code = include_bytes!("../snippets/golden.bf");
//	let code = include_bytes!("../snippets/life.bf");
	
	// Lex code
	let toks = parse(code)?;
	let toks = remove_frontmatter_tokens(&toks);
	
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
					_ = writeln!(c, "print_now(o, {out_buf_offset});");
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
				if frame.curr_offset != 0 {
					_ = writeln!(c, "c = c{};", print_offset(frame.curr_offset));
				}
				frame.curr_offset = 0; // reset
				
				// Materialize out buffer
				if out_buf_offset > 0 {
					_ = writeln!(c, "print_now(o, {out_buf_offset});");
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
				if frame.curr_offset != 0 {
					_ = writeln!(c, "c = c{};", print_offset(frame.curr_offset));
				}
				
				// Materialize out buffer
				if out_buf_offset > 0 {
					_ = writeln!(c, "print_now(o, {out_buf_offset});");
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
		_ = writeln!(c, "print_now(o, {out_buf_offset});");
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
