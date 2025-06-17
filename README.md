# Hash Index RS

A cross-platform tool for generating 64-bit (also 128-bits) non-cryptographic hashes for all files in a given directory with the purpose in mind to help in file deduplication tasks.

## Warnings

- Alpha version: **This crate is a work in progress**
- This tool is not collision resistant but it is fast.
- The default generated hash is a **64-bit non-cryptographic hash**: It is not suitable for security-sensitive applications. Use cryptographic hashing libraries (e.g., SHA-256) if you require secure hashing. Optionally you can use a 128-bit non-cryptographic hash.


## Overview

This tool provides an utility to recursively explore a directory and generate 64-bit hashes for each file. It is designed for lightweight use cases such as file comparison or integrity checks, but **not** for cryptographic purposes.

## Features

- **Fast Hashing**: Generates 64-bit hashes quickly for large directories (See [xxh64](https://crates.io/crates/xxhash-rust)).
- **Cross-Platform**: It should work on Linux, Windows, and Mac(Not tested)?.
- **Cross-Architecture**: It should work on x64, arm64(Not tested)?.
- **Minimal Memory footprint**: Uses buffered reading to minimize memory usage.
- **Async multi task**: Runs a number of tasks in parallel matching the cores in your system to speed up the process.
- **Recursive Exploration**: Scans all files in subdirectories keeping the heap footprint small by avoiding recursive function calls.

## Command Line Usage

`$hashindex-rs --help` output:
```
Usage: hashindex-rs <base_path> <label> [-d <delimiter>] [-h <hash-list>] [-j <jobs>] [-v]

HASHINDEX-RS is a tool to hash all the contained files in the provided path.

Features:

 - It sends to stdout the results in comma separated as [label], [hash(s)], [size (kbs)], [path]

 - It runs a number of tasks equal to the number of cores of the system

 - It ignores links

 - See the optional arguments for possible customisations

`

Warning: The hash created are not cryptographically strong It calculates a 64 bit hash for each item.

Warning: This tool will not follow links.

Warning: The order of the hash map presented will not necesarily be deterministic

Positional Arguments:
  base_path         the base path to explore
  label             the label for the dataset is mandatory

Options:
  -d, --delimiter   the field delitimer. It will accept a string
  -h, --hash-list   list of hash algorithms to use. default algorithm `xxh3`.
                    Order matters choose from xxh64, xxh3. use comma separater
                    list such as --hash-list xxh64,xxh3 or --hash-list "xxh64,
                    xxh3"
  -j, --jobs        number of jobs to use to compute hashes. defaults to the
                    number of cores
  -v, --version     prints the version of the application and exits. It will
                    ignore any other parameter
  --help, help      display usage information
```

## Library Usage

The core functionality is provided by the `explore_path` and `run_workers` functions in the library, which explores directories and discovered paths assynchronously.

the `hasher_wrapper` module provides a way to add more hashers that allow stream operation.
