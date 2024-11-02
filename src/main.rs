use std::{
	error::Error,
	fmt::{self, Display},
	fs::{self, File},
	io,
	path::{Path, PathBuf},
	time::{Instant, SystemTime},
};

use clap::{Parser, ValueEnum};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use regex::Regex;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about)]
struct Args {
	/// Specifies the directory to organise
	#[arg(short, long)]
	dir: PathBuf,

	/// Specifies the organisation mode
	#[arg(short, long, value_enum, default_value_t=Mode::Fast)]
	mode: Mode,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Mode {
	/// Indicates that quick (shallow) comparisons of files should be performed.
	/// Only files that appear to have not been organised, based on their name, are included in the scanning process.
	Fast,

	/// Indicates that slow (deep) comparisons of files should be performed.
	/// All files, regardless of their name, are included in the scanning process.
	Full,
}

fn main() {
	let args = Args::parse();

	match organise(&args.dir, args.mode.into()) {
		Ok(()) => println!("Successfully organised directory."),
		Err(err) => println!("Failed to organise directory: {}.", err),
	};
}

#[derive(Debug)]
pub enum OrganiseError {
	FailedToList(io::Error),
	FailedToRead(io::Error),
	FailedToRemoveDuplicate(io::Error),
	FailedToRenameNew(io::Error),
	FailedToSetLastModified(io::Error),
}

pub type OrganiseResult = Result<(), OrganiseError>;

impl Error for OrganiseError {}

impl Display for OrganiseError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::FailedToList(e) => write!(f, "failed to list files [{}]", e),
			Self::FailedToRead(e) => write!(f, "failed to read file [{}]", e),
			Self::FailedToRemoveDuplicate(e) => write!(f, "failed to remove duplicate file [{}]", e),
			Self::FailedToRenameNew(e) => write!(f, "failed to rename new file [{}]", e),
			Self::FailedToSetLastModified(e) => write!(f, "failed to set last modified time on file [{}]", e),
		}
	}
}

fn organise<T>(dir: T, mode: Mode) -> OrganiseResult
where
	T: AsRef<Path>,
{
	println!("Discovering files in <{}>...", dir.as_ref().display());

	let start = Instant::now();
	let pattern = Regex::new("^[a-f0-9]{32}$").unwrap();

	let files = fs::read_dir(&dir).map_err(OrganiseError::FailedToList)?.flatten().map(|d| d.path());

	#[rustfmt::skip]
	let files: Vec<PathBuf> = match mode {
		Mode::Full => files.collect(),
		Mode::Fast => files
			.filter(|p| {
				p.file_stem()
					.and_then(|n| n.to_str())
					.map(|n| !pattern.is_match(n))
					.unwrap_or(true)
			})
			.collect(),
	};

	println!("Discovered {} files in {:#?}.", files.len(), start.elapsed());
	println!("Organising {} files...", files.len());

	files.into_par_iter().for_each(|file| {
		if let Err(e) = process(&file) {
			println!("Failed to organise file <{}>: {}.", file.display(), e);
		}
	});

	Ok(())
}

fn process<T>(file: T) -> OrganiseResult
where
	T: AsRef<Path>,
{
	let contents = fs::read(&file).map_err(OrganiseError::FailedToRead)?;

	let checksum = format!("{:x}", md5::compute(contents));
	let checksum_file = {
		let base = file.as_ref().with_file_name(checksum);
		let full = file.as_ref().extension().map(|e| base.with_extension(e)).unwrap_or(base);

		full
	};

	if checksum_file == file.as_ref() {
		return Ok(());
	}

	if checksum_file.try_exists().map_err(OrganiseError::FailedToRead)? {
		println!("Deleting duplicate file <{}>...", file.as_ref().display());

		let time = file.as_ref().metadata().and_then(|m| m.modified()).unwrap_or(SystemTime::now());

		fs::remove_file(file).map_err(OrganiseError::FailedToRemoveDuplicate)?;

		File::options()
			.write(true)
			.open(checksum_file)
			.and_then(|f| f.set_modified(time))
			.map_err(OrganiseError::FailedToSetLastModified)?;
	} else {
		println!("Organising new file <{}>...", file.as_ref().display());

		fs::rename(file, checksum_file).map_err(OrganiseError::FailedToRenameNew)?;
	}

	Ok(())
}
