
pub struct PatchIterMut<'a, T> {
	pub vec: &'a mut Vec<T>,
	next_idx: usize,
	has_patched_curr_elem: bool,
}

impl<'a, T> PatchIterMut<'a, T> {
	pub fn new(vec: &'a mut Vec<T>) -> Self {
		Self {
			vec,
			next_idx: 0,
			has_patched_curr_elem: false,
		}
	}
	
	/// Gets the last yielded element again
	/// 
	/// Panics if the iterator hasn't yielded any elements (`next()`
	/// was never called) or the iterator is exhausted 
	pub fn get_again(&mut self) -> &mut T {
		let idx = self.curr_idx();
		&mut self.vec[idx]
	}
	
	/// Panics if the idx is invalid
	pub fn curr_idx(&self) -> usize {
		let idx = self.next_idx.checked_sub(1).expect("Never called next()");
		if idx >= self.vec.len() {
			panic!("Iterator exhausted");
		}
		idx
	}
	
	pub fn patch_elem(&mut self, new_elems: impl IntoIterator<Item = T>) {
		assert_eq!(self.has_patched_curr_elem, false);
		self.has_patched_curr_elem = true;
		
		let curr_idx = self.curr_idx();
		let prev_len = self.vec.len();
		
		self.vec.splice(curr_idx..curr_idx+1, new_elems);
		let new_len = self.vec.len();
		
		// We shrunk
		if prev_len > new_len {
			self.next_idx -= prev_len - new_len;
		}
		else if prev_len < new_len {
			self.next_idx += new_len - prev_len;
		}
	}
	
	pub fn next(&mut self) -> Option<&mut T> {
		self.has_patched_curr_elem = false;
		
		let elem = self.vec.get_mut(self.next_idx)?;
		self.next_idx += 1;
		Some(elem)
	}
}

// Doesn't work because lifetime shit (would need LendingIterator)
//impl<'a, T> Iterator for PatchIterMut<'a, T> {
//	type Item = &'a mut T;
//	
//	fn next(&mut self) -> Option<Self::Item> {
//		self.has_patched_curr_elem = false;
//		
//		let elem = self.data.get_mut(self.next_idx)?;
//		self.next_idx += 1;
//		Some(elem)
//	}
//}
