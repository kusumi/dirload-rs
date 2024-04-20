mod dir;
mod flist;
mod stat;
mod util;
mod worker;

const VERSION: [i32; 3] = [0, 1, 4];

#[derive(Debug)]
struct Opt {
    num_set: usize,
    num_reader: usize,
    num_writer: usize,
    num_repeat: isize,
    time_minute: u64,
    time_second: u64,
    monitor_int_minute: u64,
    monitor_int_second: u64,
    stat_only: bool,
    ignore_dot: bool,
    follow_symlink: bool,
    read_buffer_size: usize,
    read_size: isize,
    write_buffer_size: usize,
    write_size: isize,
    random_write_data: bool,
    num_write_paths: isize,
    truncate_write_paths: bool,
    fsync_write_paths: bool,
    dirsync_write_paths: bool,
    keep_write_paths: bool,
    clean_write_paths: bool,
    write_paths_base: String,
    write_paths_type: Vec<dir::WritePathsType>,
    path_iter: worker::PathIter,
    flist_file: String,
    flist_file_create: bool,
    force: bool,
    verbose: bool,
    debug: bool,
}

impl Default for Opt {
    fn default() -> Opt {
        Opt {
            num_set: 1,
            num_reader: 0,
            num_writer: 0,
            num_repeat: -1,
            time_minute: 0,
            time_second: 0,
            monitor_int_minute: 0,
            monitor_int_second: 0,
            stat_only: false,
            ignore_dot: false,
            follow_symlink: false,
            read_buffer_size: 1 << 16,
            read_size: -1,
            write_buffer_size: 1 << 16,
            write_size: -1,
            random_write_data: false,
            num_write_paths: 1 << 10,
            truncate_write_paths: false,
            fsync_write_paths: false,
            dirsync_write_paths: false,
            keep_write_paths: false,
            clean_write_paths: false,
            write_paths_base: "x".to_string(),
            write_paths_type: vec![dir::WritePathsType::Dir, dir::WritePathsType::Reg],
            path_iter: worker::PathIter::Ordered,
            flist_file: String::new(),
            flist_file_create: false,
            force: false,
            verbose: false,
            debug: false,
        }
    }
}

fn get_version_string() -> String {
    format!("{}.{}.{}", VERSION[0], VERSION[1], VERSION[2])
}

fn print_version() {
    println!("{}", get_version_string());
}

fn usage(progname: &str, opts: &getopts::Options) {
    print!(
        "{}",
        opts.usage(&format!("usage: {progname} [<options>] <paths>"))
    );
}

fn init_log(f: &str) {
    simplelog::CombinedLogger::init(vec![simplelog::WriteLogger::new(
        simplelog::LevelFilter::Info,
        simplelog::Config::default(),
        std::fs::File::create(f).unwrap(),
    )])
    .unwrap();
    assert!(std::path::Path::new(&f).is_file());
}

static mut INTERRUPTED: bool = false;

extern "C" fn sigint_handler(_: libc::c_int) {
    log::info!("{}: SIGINT", stringify!(sigint_handler));
    unsafe {
        INTERRUPTED = true;
    }
}

fn is_interrupted() -> bool {
    unsafe { INTERRUPTED }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let progname = args[0].clone();

    let mut opts = getopts::Options::new();
    opts.optopt("", "num_set", "Number of sets to run (default 1)", "<uint>");
    opts.optopt("", "num_reader", "Number of reader threads", "<uint>");
    opts.optopt("", "num_writer", "Number of writer threads", "<uint>");
    opts.optopt(
        "",
        "num_repeat",
        "Exit threads after specified iterations if > 0 (default -1)",
        "<int>",
    );
    opts.optopt(
        "",
        "time_minute",
        "Exit threads after sum of this and -time_second option if > 0",
        "<uint>",
    );
    opts.optopt(
        "",
        "time_second",
        "Exit threads after sum of this and -time_minute option if > 0",
        "<uint>",
    );
    opts.optopt(
        "",
        "monitor_interval_minute",
        "Monitor threads every sum of this and -monitor_interval_second option if > 0",
        "<uint>",
    );
    opts.optopt(
        "",
        "monitor_interval_second",
        "Monitor threads every sum of this and -monitor_interval_minute option if > 0",
        "<uint>",
    );
    opts.optflag("", "stat_only", "Do not read file data");
    opts.optflag("", "ignore_dot", "Ignore entries start with .");
    opts.optflag(
        "",
        "follow_symlink",
        "Follow symbolic links for read unless directory",
    );
    opts.optopt(
        "",
        "read_buffer_size",
        "Read buffer size (default 65536)",
        "<uint>",
    );
    opts.optopt(
        "",
        "read_size",
        "Read residual size per file read, use < read_buffer_size random size if 0 (default -1)",
        "<int>",
    );
    opts.optopt(
        "",
        "write_buffer_size",
        "Write buffer size (default 65536)",
        "<uint>",
    );
    opts.optopt(
        "",
        "write_size",
        "Write residual size per file write, use < write_buffer_size random size if 0 (default -1)",
        "<int>",
    );
    opts.optflag("", "random_write_data", "Use pseudo random write data");
    opts.optopt(
        "",
        "num_write_paths",
        "Exit writer threads after creating specified files or directories if > 0 (default 1024)",
        "<int>",
    );
    opts.optflag(
        "",
        "truncate_write_paths",
        "ftruncate(2) write paths for regular files instead of write(2)",
    );
    opts.optflag("", "fsync_write_paths", "fsync(2) write paths");
    opts.optflag(
        "",
        "dirsync_write_paths",
        "fsync(2) parent directories of write paths",
    );
    opts.optflag(
        "",
        "keep_write_paths",
        "Do not unlink write paths after writer threads exit",
    );
    opts.optflag(
        "",
        "clean_write_paths",
        "Unlink existing write paths and exit",
    );
    opts.optopt(
        "",
        "write_paths_base",
        "Base name for write paths (default x)",
        "<string>",
    );
    opts.optopt(
        "",
        "write_paths_type",
        "File types for write paths [d|r|s|l] (default dr)",
        "<string>",
    );
    opts.optopt(
        "",
        "path_iter",
        "<paths> iteration type [walk|ordered|reverse|random] (default ordered)",
        "<string>",
    );
    opts.optopt("", "flist_file", "Path to flist file", "<string>");
    opts.optflag("", "flist_file_create", "Create flist file and exit");
    opts.optflag("", "force", "Enable force mode");
    opts.optflag("", "verbose", "Enable verbose print");
    opts.optflag("", "debug", "Create debug log file under home directory");
    opts.optflag("v", "version", "Print version and exit");
    opts.optflag("h", "help", "Print usage and exit");

    let matches = opts.parse(&args[1..]).unwrap();
    if matches.opt_present("v") {
        print_version();
        std::process::exit(1);
    }
    if matches.opt_present("h") {
        usage(&progname, &opts);
        std::process::exit(1);
    }

    let mut opt = Opt {
        ..Default::default()
    };
    if matches.opt_present("num_set") {
        opt.num_set = matches.opt_str("num_set").unwrap().parse().unwrap();
    }
    if matches.opt_present("num_reader") {
        opt.num_reader = matches.opt_str("num_reader").unwrap().parse().unwrap();
    }
    if matches.opt_present("num_writer") {
        opt.num_writer = matches.opt_str("num_writer").unwrap().parse().unwrap();
    }
    if matches.opt_present("num_repeat") {
        opt.num_repeat = matches.opt_str("num_repeat").unwrap().parse().unwrap();
        if opt.num_repeat == 0 || opt.num_repeat < -1 {
            opt.num_repeat = -1;
        }
    }
    if matches.opt_present("time_minute") {
        opt.time_minute = matches.opt_str("time_minute").unwrap().parse().unwrap();
    }
    if matches.opt_present("time_second") {
        opt.time_second = matches.opt_str("time_second").unwrap().parse().unwrap();
    }
    opt.time_second += opt.time_minute * 60;
    opt.time_minute = 0;
    if matches.opt_present("monitor_interval_minute") {
        opt.monitor_int_minute = matches
            .opt_str("monitor_interval_minute")
            .unwrap()
            .parse()
            .unwrap();
    }
    if matches.opt_present("monitor_interval_second") {
        opt.monitor_int_second = matches
            .opt_str("monitor_interval_second")
            .unwrap()
            .parse()
            .unwrap();
    }
    opt.monitor_int_second += opt.monitor_int_minute * 60;
    opt.monitor_int_minute = 0;
    opt.stat_only = matches.opt_present("stat_only");
    opt.ignore_dot = matches.opt_present("ignore_dot");
    opt.follow_symlink = matches.opt_present("follow_symlink");
    if matches.opt_present("read_buffer_size") {
        opt.read_buffer_size = matches
            .opt_str("read_buffer_size")
            .unwrap()
            .parse()
            .unwrap();
        if opt.read_buffer_size > dir::MAX_BUFFER_SIZE {
            println!("Invalid read buffer size {}", opt.read_buffer_size);
            std::process::exit(1);
        }
    }
    if matches.opt_present("read_size") {
        opt.read_size = matches.opt_str("read_size").unwrap().parse().unwrap();
        if opt.read_size < -1 {
            opt.read_size = -1;
        } else if opt.read_size > dir::MAX_BUFFER_SIZE.try_into().unwrap() {
            println!("Invalid read size {}", opt.read_size);
            std::process::exit(1);
        }
    }
    if matches.opt_present("write_buffer_size") {
        opt.write_buffer_size = matches
            .opt_str("write_buffer_size")
            .unwrap()
            .parse()
            .unwrap();
        if opt.write_buffer_size > dir::MAX_BUFFER_SIZE {
            println!("Invalid write buffer size {}", opt.write_buffer_size);
            std::process::exit(1);
        }
    }
    if matches.opt_present("write_size") {
        opt.write_size = matches.opt_str("write_size").unwrap().parse().unwrap();
        if opt.write_size < -1 {
            opt.write_size = -1;
        } else if opt.write_size > dir::MAX_BUFFER_SIZE.try_into().unwrap() {
            println!("Invalid write size {}", opt.write_size);
            std::process::exit(1);
        }
    }
    opt.random_write_data = matches.opt_present("random_write_data");
    if matches.opt_present("num_write_paths") {
        opt.num_write_paths = matches.opt_str("num_write_paths").unwrap().parse().unwrap();
        if opt.num_write_paths < -1 {
            opt.num_write_paths = -1;
        }
    }
    opt.truncate_write_paths = matches.opt_present("truncate_write_paths");
    opt.fsync_write_paths = matches.opt_present("fsync_write_paths");
    opt.dirsync_write_paths = matches.opt_present("dirsync_write_paths");
    opt.keep_write_paths = matches.opt_present("keep_write_paths");
    opt.clean_write_paths = matches.opt_present("clean_write_paths");
    if matches.opt_present("write_paths_base") {
        opt.write_paths_base = matches.opt_str("write_paths_base").unwrap();
        if opt.write_paths_base.is_empty() {
            println!("Empty write paths base");
            std::process::exit(1);
        }
        if let Ok(v) = opt.write_paths_base.parse::<usize>() {
            opt.write_paths_base = "x".repeat(v);
            println!("Using base name {} for write paths", opt.write_paths_base);
        }
    }
    if matches.opt_present("write_paths_type") {
        let s = matches.opt_str("write_paths_type").unwrap();
        if s.is_empty() {
            println!("Empty write paths type");
            std::process::exit(1);
        }
        opt.write_paths_type.clear();
        for x in s.chars() {
            opt.write_paths_type.push(match x {
                'd' => dir::WritePathsType::Dir,
                'r' => dir::WritePathsType::Reg,
                's' => dir::WritePathsType::Symlink,
                'l' => dir::WritePathsType::Link,
                v => {
                    println!("Invalid write paths type {v}");
                    std::process::exit(1);
                }
            });
        }
    }
    if matches.opt_present("path_iter") {
        opt.path_iter = match matches.opt_str("path_iter").unwrap().as_str() {
            "walk" => worker::PathIter::Walk,
            "ordered" => worker::PathIter::Ordered,
            "reverse" => worker::PathIter::Reverse,
            "random" => worker::PathIter::Random,
            v => {
                println!("Invalid path iteration type {v}");
                std::process::exit(1);
            }
        };
    }
    if matches.opt_present("flist_file") {
        opt.flist_file = matches.opt_str("flist_file").unwrap();
    }
    // using flist file means not walking input directories
    if !opt.flist_file.is_empty() && opt.path_iter.is_walk() {
        opt.path_iter = worker::PathIter::Ordered;
        println!("Using flist, force --path_iter=ordered");
    }
    opt.flist_file_create = matches.opt_present("flist_file_create");
    opt.force = matches.opt_present("force");
    opt.verbose = matches.opt_present("verbose");
    opt.debug = matches.opt_present("debug");

    if cfg!(target_os = "windows") {
        assert!(util::is_windows());
        println!("Windows unsupported");
        std::process::exit(1);
    }

    let s = util::get_path_separator();
    if s != '/' {
        println!("Invalid path separator {s}");
        std::process::exit(1);
    }

    if matches.free.is_empty() {
        usage(&progname, &opts);
        std::process::exit(1);
    }

    let home = dirs::home_dir()
        .unwrap()
        .into_os_string()
        .into_string()
        .unwrap();
    if opt.debug {
        init_log(&util::join_path(&home, ".dirload.log"));
        log::info!("{opt:?}");
    }

    // only allow directories since now that write is supported
    let args = matches.free;
    let mut input = vec![];
    for v in &args {
        let absf = util::get_abspath(v).unwrap();
        assert!(!absf.ends_with('/'));
        if !util::get_raw_file_type(&absf).unwrap().is_dir() {
            println!("{absf} not directory");
            std::process::exit(1);
        }
        if !opt.force {
            let mut count = 0;
            for x in absf.chars() {
                if x == '/' {
                    count += 1;
                }
            }
            // /path/to/dir is allowed, but /path/to is not
            if count < 3 {
                println!("{absf} not allowed, use --force option to proceed");
                std::process::exit(1);
            }
        }
        input.push(absf);
    }
    log::info!("input {input:?}");

    // and the directories should be writable
    if opt.debug && opt.num_writer > 0 {
        for f in &input {
            log::info!("{} writable {}", f, util::is_dir_writable(f).unwrap());
        }
    }

    // create flist and exit
    if opt.flist_file_create {
        if opt.flist_file.is_empty() {
            println!("Empty flist file path");
            std::process::exit(1);
        }
        flist::create_flist_file(&input, &opt.flist_file, opt.ignore_dot, opt.force).unwrap();
        println!("{:?}", util::path_exists_or_error(&opt.flist_file).unwrap());
        std::process::exit(0);
    }
    // clean write paths and exit
    if opt.clean_write_paths {
        let mut l = dir::collect_write_paths(&input, &opt).unwrap();
        let a = l.len();
        dir::unlink_write_paths(&mut l, -1).unwrap();
        let b = l.len();
        assert!(a >= b);
        println!("Unlinked {} / {} write paths", a - b, a);
        if b != 0 {
            println!("{b} / {a} write paths remaining");
            std::process::exit(1);
        }
        std::process::exit(0);
    }

    unsafe {
        libc::signal(libc::SIGINT, sigint_handler as usize);
    }

    // ready to dispatch workers
    for i in 0..opt.num_set {
        if opt.num_set != 1 {
            println!("{}", "=".repeat(80));
            let s = format!("Set {}/{}", i + 1, opt.num_set);
            println!("{s}");
            log::info!("{s}");
        }
        let (_, num_interrupted, num_error, num_remain, tsv) =
            match worker::dispatch_worker(&input, &opt) {
                Ok(v) => v,
                Err(e) => panic!("{e}"),
            };
        if num_interrupted > 0 {
            let mut s = "";
            if num_interrupted > 1 {
                s = "s";
            }
            println!("{num_interrupted} worker{s} interrupted");
        }
        if num_error > 0 {
            let mut s = "";
            if num_error > 1 {
                s = "s";
            }
            println!("{num_error} worker{s} failed");
        }
        if num_remain > 0 {
            let mut s = "";
            if num_remain > 1 {
                s = "s";
            }
            println!("{num_remain} write path{s} remaining");
        }
        stat::print_stat(&tsv);
        if num_interrupted > 0 {
            break;
        } else if opt.num_set != 1 && i != opt.num_set - 1 {
            println!();
        }
    }
}
