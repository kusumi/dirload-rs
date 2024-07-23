dirload-rs ([v0.1.5](https://github.com/kusumi/dirload-rs/releases/tag/v0.1.5))
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

## Usage

    $ ./target/release/dirload-rs
    usage: ./target/release/dirload-rs [<options>] <paths>
    
    Options:
            --num_set <uint>
                            Number of sets to run (default 1)
            --num_reader <uint>
                            Number of reader threads
            --num_writer <uint>
                            Number of writer threads
            --num_repeat <int>
                            Exit threads after specified iterations if > 0
                            (default -1)
            --time_minute <uint>
                            Exit threads after sum of this and -time_second option
                            if > 0
            --time_second <uint>
                            Exit threads after sum of this and -time_minute option
                            if > 0
            --monitor_interval_minute <uint>
                            Monitor threads every sum of this and
                            -monitor_interval_second option if > 0
            --monitor_interval_second <uint>
                            Monitor threads every sum of this and
                            -monitor_interval_minute option if > 0
            --stat_only     Do not read file data
            --ignore_dot    Ignore entries start with .
            --follow_symlink
                            Follow symbolic links for read unless directory
            --read_buffer_size <uint>
                            Read buffer size (default 65536)
            --read_size <int>
                            Read residual size per file read, use <
                            read_buffer_size random size if 0 (default -1)
            --write_buffer_size <uint>
                            Write buffer size (default 65536)
            --write_size <int>
                            Write residual size per file write, use <
                            write_buffer_size random size if 0 (default -1)
            --random_write_data
                            Use pseudo random write data
            --num_write_paths <int>
                            Exit writer threads after creating specified files or
                            directories if > 0 (default 1024)
            --truncate_write_paths
                            ftruncate(2) write paths for regular files instead of
                            write(2)
            --fsync_write_paths
                            fsync(2) write paths
            --dirsync_write_paths
                            fsync(2) parent directories of write paths
            --keep_write_paths
                            Do not unlink write paths after writer threads exit
            --clean_write_paths
                            Unlink existing write paths and exit
            --write_paths_base <string>
                            Base name for write paths (default x)
            --write_paths_type <string>
                            File types for write paths [d|r|s|l] (default dr)
            --path_iter <string>
                            <paths> iteration type [walk|ordered|reverse|random]
                            (default ordered)
            --flist_file <string>
                            Path to flist file
            --flist_file_create
                            Create flist file and exit
            --force         Enable force mode
            --verbose       Enable verbose print
            --debug         Create debug log file under home directory
        -v, --version       Print version and exit
        -h, --help          Print usage and exit
