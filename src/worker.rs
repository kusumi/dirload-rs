use crate::dir;
use crate::flist;
use crate::is_interrupted;
use crate::stat;
use crate::util;
use crate::UserOpt;

pub const PATH_ITER_WALK: usize = 0;
pub const PATH_ITER_ORDERED: usize = 1;
pub const PATH_ITER_REVERSE: usize = 2;
pub const PATH_ITER_RANDOM: usize = 3;

#[derive(Debug, Default)]
pub struct Thread {
    pub gid: usize,
    pub dir: dir::ThreadDir,
    pub stat: stat::ThreadStat,
    num_complete: usize,
    num_interrupted: usize,
    num_error: usize,
}

pub fn newread(gid: usize, bufsiz: usize) -> Thread {
    Thread {
        gid,
        dir: dir::newread(bufsiz),
        stat: stat::newread(),
        ..Default::default()
    }
}

pub fn newwrite(gid: usize, bufsiz: usize) -> Thread {
    Thread {
        gid,
        dir: dir::newwrite(bufsiz),
        stat: stat::newwrite(),
        ..Default::default()
    }
}

pub fn is_reader(thr: &Thread, opt: &UserOpt) -> bool {
    thr.gid < opt.num_reader
}

pub fn is_writer(thr: &Thread, opt: &UserOpt) -> bool {
    !is_reader(thr, opt)
}

fn setup_flist_impl(input: &[String], opt: &UserOpt) -> std::io::Result<Vec<Vec<String>>> {
    let mut fls: Vec<Vec<String>> = vec![];
    for _ in 0..input.len() {
        fls.push(vec![]);
    }

    if !opt.flist_file.is_empty() {
        // load flist from flist file
        assert!(opt.path_iter != PATH_ITER_WALK);
        println!("flist_file {}", opt.flist_file);
        let l = flist::load_flist_file(&opt.flist_file)?;
        for s in l.iter() {
            let mut found = false;
            for (i, f) in input.iter().enumerate() {
                if s.starts_with(f) {
                    fls[i].push(s.to_string());
                    found = true;
                    // no break, s can exist in multiple fls[i]
                }
            }
            if !found {
                println!("{} has no prefix in {:?}", s, input);
                return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
            }
        }
    } else {
        // initialize flist by walking input directories
        for (i, f) in input.iter().enumerate() {
            let l = flist::init_flist(f, opt.ignore_dot)?;
            println!("{} files scanned from {}", l.len(), f);
            fls[i] = l;
        }
    }

    // don't allow empty flist as it results in spinning loop
    for (i, fl) in fls.iter().enumerate() {
        if !fl.is_empty() {
            println!("flist {} {}", input[i], fl.len());
        } else {
            println!("empty flist {}", input[i]);
            return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
        }
    }
    Ok(fls)
}

fn setup_flist(input: &[String], opt: &UserOpt) -> std::io::Result<Vec<Vec<String>>> {
    // setup flist for non-walk iterations
    if opt.path_iter == PATH_ITER_WALK {
        for f in input.iter() {
            println!("Walk {}", f);
        }
        Ok(vec![])
    } else {
        let fls = setup_flist_impl(input, opt)?;
        assert!(input.len() == fls.len());
        Ok(fls)
    }
}

fn debug_print_complete(repeat: isize, thr: &Thread, opt: &UserOpt) {
    let t = if is_reader(thr, opt) {
        "reader"
    } else {
        "writer"
    };
    let msg = format!(
        "#{} {} complete - repeat {} iswritedone {}",
        thr.gid,
        t,
        repeat,
        dir::is_write_done(thr, opt)
    );
    log::info!("{}", msg);
    if opt.debug {
        println!("{}", msg);
    }
}

fn thread_handler(
    input_path: &str,
    fl: Option<&Vec<String>>,
    thr: &mut Thread,
    dir: &dir::Dir,
    opt: &UserOpt,
) -> std::io::Result<()> {
    let iter_walk = opt.path_iter == PATH_ITER_WALK;
    let duration = opt.time_minute * 60 + opt.time_second;
    let mut repeat = 0;

    // assert thr
    assert!(thr.num_complete == 0);
    assert!(thr.num_interrupted == 0);
    assert!(thr.num_error == 0);

    // start loop
    thr.stat.set_input_path(input_path);

    // Note that PATH_ITER_WALK can fall into infinite loop when used
    // in conjunction with writer or symlink.
    loop {
        // either walk or select from input path
        if iter_walk {
            for entry in walkdir::WalkDir::new(input_path)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let f = util::parse_walkdir_entry(&entry)?;
                assert!(f.starts_with(input_path));
                if is_reader(thr, opt) {
                    dir::read_entry(f, thr, opt)?;
                } else {
                    dir::write_entry(f, thr, dir, opt)?;
                }
                if is_interrupted() {
                    thr.num_interrupted += 1;
                    break;
                }
                if duration > 0 && thr.stat.time_elapsed() > duration {
                    thr.num_complete += 1;
                    break;
                }
            }
        } else {
            let fl = fl.unwrap();
            for i in 0..fl.len() {
                let idx = match opt.path_iter {
                    PATH_ITER_ORDERED => i,
                    PATH_ITER_REVERSE => fl.len() - 1 - i,
                    PATH_ITER_RANDOM => util::get_random(0..fl.len()),
                    _ => return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput)),
                };
                let f = &fl[idx];
                assert!(f.starts_with(input_path));
                if is_reader(thr, opt) {
                    dir::read_entry(f, thr, opt)?;
                } else {
                    dir::write_entry(f, thr, dir, opt)?;
                }
                if is_interrupted() {
                    thr.num_interrupted += 1;
                    break;
                }
                if duration > 0 && thr.stat.time_elapsed() > duration {
                    thr.num_complete += 1;
                    break;
                }
            }
        }
        // return if interrupted or complete
        if thr.num_interrupted > 0 || thr.num_complete > 0 {
            return Ok(()); // not break
        }
        // otherwise continue until num_repeat if specified
        thr.stat.inc_num_repeat();
        repeat += 1;
        if opt.num_repeat > 0 && repeat >= opt.num_repeat {
            break; // usually only readers break from here
        }
        if is_writer(thr, opt) && dir::is_write_done(thr, opt) {
            break;
        }
    }

    if is_reader(thr, opt) {
        assert!(opt.num_repeat > 0);
        assert!(repeat >= opt.num_repeat);
    } else {
        assert!(dir::is_write_done(thr, opt));
    }

    debug_print_complete(repeat, thr, opt);
    thr.num_complete += 1;

    Ok(())
}

pub fn dispatch_worker(
    input: &[String],
    opt: &UserOpt,
) -> std::io::Result<(usize, usize, usize, usize, Vec<stat::ThreadStat>)> {
    for f in input.iter() {
        assert!(util::is_abspath(f));
    }

    // number of readers and writers are 0 by default
    if opt.num_reader == 0 && opt.num_writer == 0 {
        return Ok((0, 0, 0, 0, vec![]));
    }

    // initialize dir
    let dir = dir::newdir(opt.random_write_data, &opt.write_paths_type);

    // initialize thread structure
    let mut thrv = vec![];
    for i in 0..opt.num_reader + opt.num_writer {
        if i < opt.num_reader {
            thrv.push(newread(i, opt.read_buffer_size));
        } else {
            thrv.push(newwrite(i, opt.write_buffer_size));
        }
    }
    assert!(thrv.len() == opt.num_reader + opt.num_writer);

    // setup flist
    let fls = setup_flist(input, opt)?;
    if opt.path_iter == PATH_ITER_WALK {
        assert!(fls.is_empty());
    } else {
        assert!(!fls.is_empty());
    }

    // spawn + join threads
    std::thread::scope(|s| {
        for thr in thrv.iter_mut() {
            log::info!("#{} start", thr.gid);
            s.spawn(|| {
                let input_path = &input[thr.gid % input.len()];
                let fl = if !fls.is_empty() {
                    Some(&fls[thr.gid % fls.len()])
                } else {
                    None
                };
                thr.stat.set_time_begin();
                if let Err(e) = thread_handler(input_path, fl, thr, &dir, opt) {
                    thr.num_error += 1;
                    log::info!("#{} {}", thr.gid, e);
                    println!("{}", e);
                }
                thr.stat.set_time_end();
            });
        }
    });

    // collect result
    let mut num_complete = 0;
    let mut num_interrupted = 0;
    let mut num_error = 0;
    for thr in thrv.iter_mut() {
        num_complete += thr.num_complete;
        num_interrupted += thr.num_interrupted;
        num_error += thr.num_error;
    }
    assert!(num_complete + num_interrupted + num_error == opt.num_reader + opt.num_writer);

    let mut tdv = vec![];
    let mut tsv = vec![];
    for thr in thrv.iter_mut() {
        tdv.push(&thr.dir);
        tsv.push(thr.stat.clone());
    }
    Ok((
        num_complete,
        num_interrupted,
        num_error,
        dir::cleanup_write_paths(tdv.as_slice(), opt)?,
        tsv,
    ))
}
