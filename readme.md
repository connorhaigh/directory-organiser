# directory-organiser

`directory-organiser` is a Rust-based command-line application that can be used to swiftly tidy up a directory of files that may contain duplicates.

## Overview

The general idea of this application is that it can be used to tidy up a directory of files that may contain duplicates by way of determining the hash of its contents and using that as the file name; that is, to say, renaming every file to be its hash and removing any files where there is a hash collision. This allows for potential duplicates with different file names to be added to which can later be rectified.

Its primary purpose is to assist in tidying up a directory of images whereby there may be duplicates added from time to time.

## Usage

Organise the current directory with sensible defaults:

```
directory-organiser --dir .
```

Organise the specified directory fully:

```
directory-organiser --dir "E:\Photos" --mode full
```
