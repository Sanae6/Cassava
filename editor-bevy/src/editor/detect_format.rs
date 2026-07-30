enum Format {
	Szs,
	Sarc,
	Byml,
	Bntx,
	Kcl,
}

pub fn detect_format(data: &[u8]) -> Option<Format> {
	match data.get(0..4)? {
		b"Yaz0" => {
    }
		_ => {}
	}
	todo!()
}
