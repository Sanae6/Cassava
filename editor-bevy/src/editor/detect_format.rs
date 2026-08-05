use std::io::Read;

use cassava::yaz0::Yaz0Stream;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Format {
	Szs,
	Sarc,
	Byml,
	Bntx,
	Kcl,
}

pub fn detect_format(data: &[u8]) -> Option<Format> {
	if data.starts_with(b"BY") || data.starts_with(b"YB") {
		return Some(Format::Byml);
	}
	match data.get(0..4)? {
		b"Yaz0" => {
			// probably szs
			let mut stream = Yaz0Stream::new(data).ok()?;
			let mut magic = [0; 4];
			stream.read_exact(&mut magic).ok()?;
			(&magic == b"SARC").then_some(Format::Szs)
		}
		b"SARC" => Some(Format::Sarc),
		[2,2,0,0] | [0,0,2,2] => Some(Format::Kcl),
		_ => match data.get(0..8)? {
			b"BNTX\0\0\0\0" => Some(Format::Bntx),
			_ => None,
		},
	}
}
