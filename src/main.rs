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

/// Performs organisation on directories.
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

/// Determines the mode of operation.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum Mode {
	/// Indicates that quick (shallow) comparisons of files based on their name should be performed.
	Fast,

	/// Indicates that slow (deep) comparisons of files based on their entire contents should be performed.
	Full,
}

fn main() {
	let args = Args::parse();

	match organise(args.dir, args.mode) {
		Ok(()) => println!("Successfully organised directory."),
		Err(err) => println!("Failed to organise directory: {}.", err),
	};
}

/// Represents an organise-related error.
#[derive(Debug)]
enum OrganiseError {
	/// Indicates that the directory could not be read for its files.
	FailedToListDirectory(io::Error),

	/// Indicates that a particular file could not be read for its contents.
	FailedToReadFile(io::Error),

	/// Indicates that a duplicate file could not be removed.
	FailedToRemoveDuplicateFile(io::Error),

	/// Indicates that a new file could not be renamed.
	FailedToRenameNewFile(io::Error),

	/// Indicates that the last modified timestamp on an original duplicate file could not be changed.
	FailedToSetLastModified(io::Error),
}

/// Indicates the result of an organisation operation.
type OrganiseResult = Result<(), OrganiseError>;

impl Display for OrganiseError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::FailedToListDirectory(e) => write!(f, "failed to list files [{}]", e),
			Self::FailedToReadFile(e) => write!(f, "failed to read file [{}]", e),
			Self::FailedToRemoveDuplicateFile(e) => write!(f, "failed to remove duplicate file [{}]", e),
			Self::FailedToRenameNewFile(e) => write!(f, "failed to rename new file [{}]", e),
			Self::FailedToSetLastModified(e) => write!(f, "failed to set last modified time on file [{}]", e),
		}
	}
}

impl Error for OrganiseError {}

/// Organises the specified directory using the specified mode.
fn organise<T>(dir: T, mode: Mode) -> OrganiseResult
where
	T: AsRef<Path>,
{
	println!("Discovering files in <{}>...", dir.as_ref().display());

	let start = Instant::now();
	let pattern = Regex::new("^[a-f0-9]{32}$").unwrap();

	let files = fs::read_dir(&dir).map_err(OrganiseError::FailedToListDirectory)?.flatten().map(|d| d.path());

	// Check either every file or only the files where the name does not appear to be a hash.

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

/// Attempts to process (organise) the specified file.
fn process<T>(file: T) -> OrganiseResult
where
	T: AsRef<Path>,
{
	let contents = fs::read(&file).map_err(OrganiseError::FailedToReadFile)?;

	let checksum = format!("{:x}", md5::compute(contents));
	let checksum_file = {
		let base = file.as_ref().with_file_name(checksum);
		let full = file.as_ref().extension().map(|e| base.with_extension(e)).unwrap_or(base);

		full
	};

	if checksum_file == file.as_ref() {
		return Ok(());
	}

	if checksum_file.try_exists().map_err(OrganiseError::FailedToReadFile)? {
		println!("Deleting duplicate file <{}>...", file.as_ref().display());

		let time = file.as_ref().metadata().and_then(|m| m.modified()).unwrap_or(SystemTime::now());

		fs::remove_file(file).map_err(OrganiseError::FailedToRemoveDuplicateFile)?;

		File::options()
			.write(true)
			.open(checksum_file)
			.and_then(|f| f.set_modified(time))
			.map_err(OrganiseError::FailedToSetLastModified)?;
	} else {
		println!("Organising new file <{}>...", file.as_ref().display());

		fs::rename(file, checksum_file).map_err(OrganiseError::FailedToRenameNewFile)?;
	}

	Ok(())
}
