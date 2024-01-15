dirload-rs ([v0.1.0](https://github.com/kusumi/dirload-rs/releases/tag/v0.1.0))
========

## About

+ Set read / write workloads on a file system.

+ Rust version of [https://github.com/kusumi/dirload](https://github.com/kusumi/dirload).

## Supported platforms

Unix-likes in general

## Requirements

Recent version of Rust

## Build

    $ make

or

    $ gmake

## Usage

    $ ./target/release/dirload-rs
    usage: ./target/release/dirload-rs [<options>] <paths>
    
    Options:
            --num_reader <uint>
                            Number of reader Goroutines
            --num_writer <uint>
                            Number of writer Goroutines
            --num_repeat <int>
                            Exit Goroutines after specified iterations if > 0
            --time_minute <uint>
                            Exit Goroutines after sum of this and -time_second
                            option if > 0
            --time_second <uint>
                            Exit Goroutines after sum of this and -time_minute
                            option if > 0
            --stat_only     Do not read file data
            --ignore_dot    Ignore entries start with .
            --lstat         Do not resolve symbolic links
            --read_buffer_size <uint>
                            Read buffer size
            --read_size <int>
                            Read residual size per file read, use <
                            read_buffer_size random size if 0
            --write_buffer_size <uint>
                            Write buffer size
            --write_size <int>
                            Write residual size per file write, use <
                            write_buffer_size random size if 0
            --random_write_data
                            Use pseudo random write data
            --num_write_paths <int>
                            Exit writer Goroutines after creating specified files
                            or directories if > 0
            --truncate_write_paths
                            ftruncate(2) write paths for regular files instead of
                            write(2)
            --fsync_write_paths
                            fsync(2) write paths
            --dirsync_write_paths
                            fsync(2) parent directories of write paths
            --keep_write_paths
                            Do not unlink write paths after writer Goroutines exit
            --clean_write_paths
                            Unlink existing write paths and exit
            --write_paths_base <string>
                            Base name for write paths
            --write_paths_type <string>
                            File types for write paths [d|r|s|l]
            --path_iter <string>
                            <paths> iteration type [walk|ordered|reverse|random]
            --flist_file <string>
                            Path to flist file
            --flist_file_create
                            Create flist file and exit
            --force         Enable force mode
            --verbose       Enable verbose print
            --debug         Create debug log file under home directory
        -v, --version       Print version and exit
        -h, --help          Print usage and exit

## Resource

[https://github.com/kusumi/dirload-rs/](https://github.com/kusumi/dirload-rs/)
