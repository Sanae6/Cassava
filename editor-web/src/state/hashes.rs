use std::{
	collections::{HashMap, HashSet},
	fs, io,
	ops::Range,
	path::{Path, PathBuf},
	sync::LazyLock,
};

use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use snafu::Snafu;
use ts_rs::TS;
use webkit6::{
	gio::{
		File, FileInputStream,
		prelude::{FileExt, InputStreamExtManual},
	},
	glib::{self, Priority},
};

use crate::state::config::Version;

type VersionVec = heapless::Vec<Version, { Version::VERSION_COUNT }>;

#[derive(Deserialize)]
pub struct Hashes {
	hashes: HashMap<PathBuf, HashMap<String, VersionVec>>,
	file_counts: HashMap<Version, usize>,
}

enum HashMatch {
	NoMatch,
	ModifiedRomfs,
	VersionBitmask(u8),
}
impl Hashes {
	fn known_path(&self, path: &Path) -> bool {
		self.hashes.contains_key(path)
	}

	fn match_version(&'static self, path: &Path, found_hash: &str) -> HashMatch {
		assert_eq!(found_hash.len(), 64);

		let Some(hashes) = self.hashes.get(path) else {
			return HashMatch::NoMatch;
		};

		let Some(matched) = hashes.get(found_hash) else {
			return HashMatch::ModifiedRomfs;
		};

		let mut bitmask = 0;
		for version in matched {
			bitmask |= 1 << *version as u8;
		}

		HashMatch::VersionBitmask(bitmask)
	}
}

static HASHES: LazyLock<Hashes> =
	LazyLock::new(|| toml::from_str::<Hashes>(include_str!("./hashes.toml")).unwrap());

#[derive(Snafu, Debug)]
pub enum HashError {
	#[snafu(transparent)]
	IO { source: io::Error },
	#[snafu(transparent)]
	GlibError { source: glib::Error },
	#[snafu(display("No matching version found for file with hash: {hash}"))]
	NoMatchingVersion { hash: String },
	// MultipleMatchingVersions {
	// 	versions: heapless::Vec<Version, 7>,
	// },
}
impl Serialize for HashError {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		format!("{}", self).serialize(serializer)
	}
}

#[derive(Serialize, TS, Debug)]
pub enum HashingReport {
	NotRomfs,
	ModifiedRomfs(PathBuf),
	MixedRomfs,
	FailedToHash(PathBuf, String),
	Version(Version),
	#[ts(as = "Vec<Version>")]
	VersionInconclusive(VersionVec),
}

pub async fn hash_romfs(
	dir: &Path,
	mut progress_report: impl FnMut(usize, usize),
) -> HashingReport {
	let mut matched_files = 0;
	let dir = walkdir::WalkDir::new(dir)
		.max_depth(4)
		.follow_links(false)
		.into_iter()
		.filter_map(|value| {
			value
				.inspect_err(|error| eprintln!("error error: {error}"))
				.ok()
				.filter(|entry| entry.file_type().is_file())
				.map(|entry| -> (PathBuf, PathBuf) {
					let path = entry.path().to_owned();
					let rel_path = entry
						.path()
						.strip_prefix(dir)
						.expect("failed to strip romfs dir prefix")
						.to_owned();
					(path, rel_path)
				})
				.filter(|(_, rel_path)| !HASHES.known_path(rel_path))
		})
		.collect::<Vec<_>>();
	let total = dir.len();
	progress_report(0, total);
	let mut possible_versions = (1u8 << std::mem::variant_count::<Version>()) - 1;
	for (full_path, rel_path) in dir {
		let result = hash_file(&full_path).await;
		match result {
			Ok(hash) => match HASHES.match_version(&rel_path, &hash) {
				HashMatch::NoMatch => {
					// allow non-dump files in romfs, as long as all of the important files are clean
				}
				HashMatch::ModifiedRomfs => return HashingReport::ModifiedRomfs(rel_path),
				HashMatch::VersionBitmask(version_bitmask) => {
					possible_versions &= version_bitmask;
					if possible_versions == 0 {
						return HashingReport::MixedRomfs;
					}
					matched_files += 1;
				}
			},
			Err(error) => {
				// println!("boo fucking womp {error}");
				return HashingReport::FailedToHash(rel_path, error.to_string());
			}
		}
		progress_report(matched_files, total);
	}

	// println!("{possible_versions:05b}");
	let mut versions = heapless::Vec::new();
	for i in 0..std::mem::variant_count::<Version>() {
		if possible_versions & 1 << i != 0 {
			versions
				.push(
					Version::from_usize(i)
						.expect("version enum discriminants are not chronologically sequential"),
				)
				.unwrap();
		}
	}
	if versions.len() > 1 {
		return HashingReport::VersionInconclusive(versions);
	}

	let version = versions[0];
	let file_count = HASHES
		.file_counts
		.get(&version)
		.expect("missing file count for version");
	if matched_files != *file_count {}

	HashingReport::Version(versions[0])
}

async fn hash_file(path: &Path) -> Result<String, HashError> {
	// println!("file: {entry:?}");
	let file = File::for_path(path);
	let input_stream = file.read_future(Priority::HIGH).await?;
	let hash = digest(&input_stream).await?;
	Ok(hash)
}

async fn digest(file: &FileInputStream) -> Result<String, glib::Error> {
	let mut hasher = Sha256::new();
	let mut buf: Box<[u8]> = Box::new([0; 4096]);
	loop {
		let (new_buf, read_count) = file
			.read_future(buf, Priority::HIGH)
			.await
			.map_err(|(_, err)| err)?;
		if read_count == 0 {
			break;
		}
		hasher.update(&new_buf[..read_count]);
		buf = new_buf;
	}
	let hash = hasher.finalize();
	let mut hash_string = String::with_capacity(64);
	for digit in hash {
		use std::fmt::Write;
		let _ = write!(&mut hash_string, "{digit:02x}");
	}
	Ok(hash_string)
}
