use anyhow::bail;

#[derive(Clone, Debug)] 
pub enum Token {
	Left(u32),
	Right(u32),
	Incr(u32),
	Decr(u32),
	Out(u32),
	In(u32),
	LoopStart,
	LoopEnd,
}

pub fn parse(text: &[u8]) -> Result<Vec<Token>, anyhow::Error> {
	macro_rules! op_with_num {
		($name:ident; $ir:expr) => {
			if let Some(Token::$name(num)) = $ir.last_mut()
				&& *num < u32::MAX
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

/// Remove frontmatter (first loop(s)) based on
/// the tokens
/// 
/// In future we will do this on the Ir since it's
/// a little simpler to do, but this can stay for
/// the old code
pub fn remove_frontmatter_tokens(toks: &[Token]) -> Vec<Token> {
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
	
	new_toks
}
