use std::num::NonZeroUsize;

pub struct Fule(Option<NonZeroUsize>);

impl Fule {
	pub fn unlimited() -> Self {
		Self(None)
	}

	pub fn limited(amount: usize) -> Self {
		Self(Some(NonZeroUsize::new(amount).unwrap_or(NonZeroUsize::MIN)))
	}

	pub fn fule(&mut self) -> bool {
		match self.0 {
			None => true,
			Some(ammount) => {
				if let Some(fule) = NonZeroUsize::new(ammount.get() - 1) {
					self.0 = Some(fule);
					true
				} else {
					false
				}
			}
		}
	}

	pub fn remainder(&self) -> Option<NonZeroUsize> {
		self.0
	}
}
