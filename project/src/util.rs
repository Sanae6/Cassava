use cassava::{sarc, yaz0};
use snafu::{ResultExt, Whatever};

pub fn load_szs(data: &[u8]) -> Result<sarc::Reader, Whatever> {
	let data = yaz0::decompress(data).whatever_context("failed to decompress szs")?;
  sarc::Reader::new(data.0).whatever_context("failed to read sarc from decompressed szs")
}
