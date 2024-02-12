use crate::dir;
use crate::flist;
use crate::is_interrupted;
use crate::stat;
use crate::util;
use crate::Opt;

pub(crate) const PATH_ITER_WALK: usize = 0;
pub(crate) const PATH_ITER_ORDERED: usize = 1;
pub(crate) const PATH_ITER_REVERSE: usize = 2;
pub(crate) const PATH_ITER_RANDOM: usize = 3;

#[derive(Debug, Default)]
pub(crate) struct Thread {
    pub(crate) gid: usize,
    pub(crate) dir: dir::ThreadDir,
    pub(crate) stat: stat::ThreadStat,
    num_complete: usize,
    num_interrupted: usize,
    num_error: usize,
    txc: Option<std::sync::mpsc::Sender<(usize, stat::ThreadStat)>>,
}

impl Thread {
    fn newread(gid: usize, bufsiz: usize) -> Thread {
        Thread {
            gid,
            dir: dir::ThreadDir::newread(bufsiz),
            stat: stat::ThreadStat::newread(),
            ..Default::default()
        }
    }

    fn newwrite(gid: usize, bufsiz: usize) -> Thread {
        Thread {
            gid,
            dir: dir::ThreadDir::newwrite(bufsiz),
            stat: stat::ThreadStat::newwrite(),
            ..Default::default()
        }
    }

    pub(crate) fn is_reader(&self, opt: &Opt) -> bool {
        self.gid < opt.num_reader
    }

    pub(crate) fn is_writer(&self, opt: &Opt) -> bool {
        !self.is_reader(opt)
    }

    fn send_stat(&mut self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        if let Some(txc) = &self.txc {
            self.stat.set_time_end();
            txc.send((self.gid, self.stat.clone()))?;
        }
        Ok(())
    }

    fn send_done(&mut self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        if let Some(txc) = &self.txc {
            self.stat.done = true;
            txc.send((self.gid, self.stat.clone()))?;
        }
        Ok(())
    }
}

fn setup_flist_impl(input: &[String], opt: &Opt) -> std::io::Result<Vec<Vec<String>>> {
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

fn setup_flist(input: &[String], opt: &Opt) -> std::io::Result<Vec<Vec<String>>> {
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

fn debug_print_complete(repeat: isize, thr: &Thread, opt: &Opt) {
    let t = if thr.is_reader(opt) {
        "reader"
    } else {
        "writer"
    };
    let msg = format!(
        "{:?} #{} {} complete - repeat {} iswritedone {}",
        std::thread::current().id(),
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

fn monitor_handler(
    n: usize,
    rxc: Option<std::sync::mpsc::Receiver<(usize, stat::ThreadStat)>>,
    opt: &Opt,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    assert!(rxc.is_some());
    assert!(opt.monitor_int_second > 0);
    let mut tsv = vec![stat::ThreadStat::new(); n];
    let mut timer = util::Timer::new(opt.monitor_int_second, 0);
    let mut ready = false;
    let rxc = rxc.unwrap();

    loop {
        let mut timeout = false;
        match rxc.recv_timeout(std::time::Duration::from_secs(1)) {
            Ok((gid, ts)) => tsv[gid] = ts,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                return Err(Box::new(std::io::Error::from(
                    std::io::ErrorKind::NotConnected,
                )));
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => timeout = true,
        };
        if !timeout {
            // exit if stats for all threads are done
            for (i, ts) in tsv.iter().enumerate() {
                if !ts.done {
                    break;
                } else if i == tsv.len() - 1 {
                    return Ok(());
                }
            }
            // confirm if stats for all threads are ready
            if !ready {
                for (i, ts) in tsv.iter().enumerate() {
                    if !ts.is_ready() {
                        break;
                    } else if i == tsv.len() - 1 {
                        ready = true;
                    }
                }
            }
        }
        if timer.elapsed() {
            let label = stringify!([monitor]);
            if ready {
                log::info!("{} ready", label);
                stat::print_stat(&tsv);
            } else {
                log::info!("{} not ready", label);
            }
            timer.reset();
        }
        // only allow existing via message by default
        if opt.debug && is_interrupted() {
            break;
        }
    }
    Ok(())
}

fn worker_handler(
    input_path: &str,
    fl: Option<&Vec<String>>,
    thr: &mut Thread,
    dir: &dir::Dir,
    opt: &Opt,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    assert!(thr.txc.is_some() || opt.monitor_int_second == 0);
    assert!(thr.txc.is_none() || opt.monitor_int_second > 0);
    let d = opt.time_second;
    let mut timer = util::Timer::new(opt.monitor_int_second, 100);
    let mut repeat = 0;

    // assert thr
    assert!(thr.num_complete == 0);
    assert!(thr.num_interrupted == 0);
    assert!(thr.num_error == 0);

    // start loop
    thr.stat.set_input_path(input_path);

    // send initial stats
    thr.send_stat()?;

    // Note that PATH_ITER_WALK can fall into infinite loop when used
    // in conjunction with writer or symlink.
    loop {
        // either walk or select from input path
        if opt.path_iter == PATH_ITER_WALK {
            for entry in walkdir::WalkDir::new(input_path)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let f = util::parse_walkdir_entry(&entry)?;
                assert!(f.starts_with(input_path));
                if thr.is_reader(opt) {
                    dir::read_entry(f, thr, opt)?;
                } else {
                    dir::write_entry(f, thr, dir, opt)?;
                }
                if is_interrupted() {
                    thr.num_interrupted += 1;
                    break;
                }
                if d > 0 && thr.stat.time_elapsed() > d {
                    debug_print_complete(repeat, thr, opt);
                    thr.num_complete += 1;
                    break;
                }
                if timer.elapsed() {
                    thr.send_stat()?;
                    timer.reset();
                }
            }
        } else {
            let fl = fl.unwrap();
            for i in 0..fl.len() {
                let idx = match opt.path_iter {
                    PATH_ITER_ORDERED => i,
                    PATH_ITER_REVERSE => fl.len() - 1 - i,
                    PATH_ITER_RANDOM => util::get_random(0..fl.len()),
                    _ => {
                        return Err(Box::new(std::io::Error::from(
                            std::io::ErrorKind::InvalidInput,
                        )))
                    }
                };
                let f = &fl[idx];
                assert!(f.starts_with(input_path));
                if thr.is_reader(opt) {
                    dir::read_entry(f, thr, opt)?;
                } else {
                    dir::write_entry(f, thr, dir, opt)?;
                }
                if is_interrupted() {
                    thr.num_interrupted += 1;
                    break;
                }
                if d > 0 && thr.stat.time_elapsed() > d {
                    debug_print_complete(repeat, thr, opt);
                    thr.num_complete += 1;
                    break;
                }
                if timer.elapsed() {
                    thr.send_stat()?;
                    timer.reset();
                }
            }
        }
        // return if interrupted or complete
        if thr.num_interrupted > 0 || thr.num_complete > 0 {
            thr.send_done()?;
            return Ok(()); // not break
        }
        // otherwise continue until num_repeat if specified
        thr.stat.inc_num_repeat();
        repeat += 1;
        if opt.num_repeat > 0 && repeat >= opt.num_repeat {
            break; // usually only readers break from here
        }
        if thr.is_writer(opt) && dir::is_write_done(thr, opt) {
            break;
        }
    }

    // send stats in case finished before sending any updates
    thr.send_stat()?;
    thr.send_done()?;

    if thr.is_reader(opt) {
        assert!(opt.num_repeat > 0);
        assert!(repeat >= opt.num_repeat);
    }
    debug_print_complete(repeat, thr, opt);
    thr.num_complete += 1;

    Ok(())
}

pub(crate) fn dispatch_worker(
    input: &[String],
    opt: &Opt,
) -> std::io::Result<(usize, usize, usize, usize, Vec<stat::ThreadStat>)> {
    for f in input.iter() {
        assert!(util::is_abspath(f));
    }
    assert!(opt.time_minute == 0);
    assert!(opt.monitor_int_minute == 0);

    // number of readers and writers are 0 by default
    if opt.num_reader == 0 && opt.num_writer == 0 {
        return Ok((0, 0, 0, 0, vec![]));
    }

    // initialize dir
    let dir = dir::Dir::new(opt.random_write_data, &opt.write_paths_type);

    // initialize thread structure
    let mut thrv = vec![];
    for i in 0..opt.num_reader + opt.num_writer {
        if i < opt.num_reader {
            thrv.push(Thread::newread(i, opt.read_buffer_size));
        } else {
            thrv.push(Thread::newwrite(i, opt.write_buffer_size));
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

    // create channels for workers to send stats to monitor
    let use_monitor = opt.monitor_int_second > 0;
    let n = thrv.len();
    let mut rxc = None;
    if use_monitor {
        let l = std::sync::mpsc::channel::<(usize, stat::ThreadStat)>();
        for thr in thrv.iter_mut() {
            thr.txc = Some(l.0.clone());
        }
        rxc = Some(l.1);
    }

    // spawn + join threads
    std::thread::scope(|s| {
        if use_monitor {
            s.spawn(|| {
                let tid = std::thread::current().id();
                log::info!("{:?} monitor start", tid);
                if let Err(e) = monitor_handler(n, rxc, opt) {
                    log::info!("{:?} {}", tid, e);
                    println!("{}", e);
                }
            });
        }
        for thr in thrv.iter_mut() {
            s.spawn(|| {
                let tid = std::thread::current().id();
                log::info!("{:?} #{} start", tid, thr.gid);
                let input_path = &input[thr.gid % input.len()];
                let fl = if !fls.is_empty() {
                    Some(&fls[thr.gid % fls.len()])
                } else {
                    None
                };
                thr.stat.set_time_begin();
                if let Err(e) = worker_handler(input_path, fl, thr, &dir, opt) {
                    thr.num_error += 1;
                    log::info!("{:?} #{} {}", tid, thr.gid, e);
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
