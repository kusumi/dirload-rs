#[derive(Clone, Debug)]
pub(crate) struct ThreadStat {
    is_reader: bool,
    input_path: String,
    time_begin: std::time::SystemTime,
    time_end: std::time::SystemTime,
    num_repeat: usize,
    num_stat: usize,
    num_read: usize,
    num_read_bytes: usize,
    num_write: usize,
    num_write_bytes: usize,
    pub(crate) done: bool,
}

impl Default for ThreadStat {
    fn default() -> ThreadStat {
        ThreadStat {
            is_reader: true,
            input_path: String::new(),
            time_begin: std::time::UNIX_EPOCH,
            time_end: std::time::UNIX_EPOCH,
            num_repeat: 0,
            num_stat: 0,
            num_read: 0,
            num_read_bytes: 0,
            num_write: 0,
            num_write_bytes: 0,
            done: false,
        }
    }
}

impl ThreadStat {
    pub(crate) fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub(crate) fn newread() -> Self {
        Self {
            is_reader: true,
            ..Default::default()
        }
    }

    pub(crate) fn newwrite() -> Self {
        Self {
            is_reader: false,
            ..Default::default()
        }
    }

    pub(crate) fn is_ready(&self) -> bool {
        !self.input_path.is_empty()
    }

    pub(crate) fn set_input_path(&mut self, f: &str) {
        self.input_path = f.to_string();
    }

    pub(crate) fn set_time_begin(&mut self) {
        self.time_begin = std::time::SystemTime::now();
    }

    pub(crate) fn set_time_end(&mut self) {
        self.time_end = std::time::SystemTime::now();
    }

    pub(crate) fn time_elapsed(&self) -> std::time::Duration {
        self.time_begin.elapsed().unwrap()
    }

    pub(crate) fn inc_num_repeat(&mut self) {
        self.num_repeat += 1;
    }

    pub(crate) fn inc_num_stat(&mut self) {
        self.num_stat += 1;
    }

    pub(crate) fn inc_num_read(&mut self) {
        self.num_read += 1;
    }

    pub(crate) fn add_num_read_bytes(&mut self, siz: usize) {
        self.num_read_bytes += siz;
    }

    pub(crate) fn inc_num_write(&mut self) {
        self.num_write += 1;
    }

    pub(crate) fn add_num_write_bytes(&mut self, siz: usize) {
        self.num_write_bytes += siz;
    }
}

pub(crate) fn print_stat(tsv: &Vec<ThreadStat>) {
    // repeat
    let mut width_repeat = "repeat".len();
    for ts in tsv {
        let s = ts.num_repeat.to_string();
        if s.len() > width_repeat {
            width_repeat = s.len();
        }
    }

    // stat
    let mut width_stat = "stat".len();
    for ts in tsv {
        let s = ts.num_stat.to_string();
        if s.len() > width_stat {
            width_stat = s.len();
        }
    }

    // read
    let mut width_read = "read".len();
    for ts in tsv {
        let s = ts.num_read.to_string();
        if s.len() > width_read {
            width_read = s.len();
        }
    }

    // read[B]
    let mut width_read_bytes = "read[B]".len();
    for ts in tsv {
        let s = ts.num_read_bytes.to_string();
        if s.len() > width_read_bytes {
            width_read_bytes = s.len();
        }
    }

    // write
    let mut width_write = "write".len();
    for ts in tsv {
        let s = ts.num_write.to_string();
        if s.len() > width_write {
            width_write = s.len();
        }
    }

    // write[B]
    let mut width_write_bytes = "write[B]".len();
    for ts in tsv {
        let s = ts.num_write_bytes.to_string();
        if s.len() > width_write_bytes {
            width_write_bytes = s.len();
        }
    }

    // sec
    let mut num_sec = vec![0f64; tsv.len()];
    for (i, t) in num_sec.iter_mut().enumerate() {
        let sec = tsv[i]
            .time_end
            .duration_since(tsv[i].time_begin)
            .unwrap()
            .as_secs_f64();
        *t = f64::trunc(sec * 100.0) / 100.0; // cut decimals
    }
    let mut width_sec = "sec".len();
    for t in &num_sec {
        let s = t.to_string();
        if s.len() > width_sec {
            width_sec = s.len();
        }
    }

    // MiB/sec
    let mut num_mibs = vec![0f64; tsv.len()];
    for (i, x) in num_mibs.iter_mut().enumerate() {
        let mib = (tsv[i].num_read_bytes + tsv[i].num_write_bytes) as f64 / f64::from(1 << 20);
        let mibs = mib / num_sec[i];
        *x = f64::trunc(mibs * 100.0) / 100.0; // cut decimals
    }
    let mut width_mibs = "MiB/sec".len();
    for x in &num_mibs {
        let s = x.to_string();
        if s.len() > width_mibs {
            width_mibs = s.len();
        }
    }

    // path
    let mut width_path = "path".len();
    for ts in tsv {
        let s = &ts.input_path;
        assert!(!s.is_empty());
        if s.len() > width_path {
            width_path = s.len();
        }
    }

    // index
    let nlines = tsv.len();
    let mut width_index = 1;
    let mut n = nlines;
    if n > 0 {
        n -= 1; // gid starts from 0
        width_index = n.to_string().len();
    }

    let mut slen = 0;
    print!("{}", " ".repeat(1 + width_index + 1));
    slen += 1 + width_index + 1;
    print!("{:<6} ", "type");
    slen += 6 + 1;
    let ls = [
        "repeat", "stat", "read", "read[B]", "write", "write[B]", "sec", "MiB/sec", "path",
    ];
    let lw = [
        width_repeat,
        width_stat,
        width_read,
        width_read_bytes,
        width_write,
        width_write_bytes,
        width_sec,
        width_mibs,
        width_path,
    ];
    for (i, s) in ls.iter().enumerate() {
        print!("{0:1$}", s, lw[i]);
        slen += lw[i];
        if i != ls.len() - 1 {
            print!(" ");
            slen += 1;
        }
    }
    println!();
    println!("{}", "-".repeat(slen));

    for i in 0..nlines {
        // index (left align)
        print!("#{i:<width_index$} ");
        // type
        if tsv[i].is_reader {
            print!("reader ");
        } else {
            print!("writer ");
        }
        // repeat
        print!("{0:>1$} ", tsv[i].num_repeat, lw[0]);
        // stat
        print!("{0:>1$} ", tsv[i].num_stat, lw[1]);
        // read
        print!("{0:>1$} ", tsv[i].num_read, lw[2]);
        // read[B]
        print!("{0:>1$} ", tsv[i].num_read_bytes, lw[3]);
        // write
        print!("{0:>1$} ", tsv[i].num_write, lw[4]);
        // write[B]
        print!("{0:>1$} ", tsv[i].num_write_bytes, lw[5]);
        // sec
        print!("{0:>1$} ", num_sec[i], lw[6]);
        // MiB/sec
        print!("{0:>1$} ", num_mibs[i], lw[7]);
        // path (left align)
        print!("{0:<1$} ", tsv[i].input_path, lw[8]);
        println!();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_newread() {
        let ts = super::ThreadStat::newread();
        assert!(ts.is_reader, "{}", ts.is_reader);
        assert_eq!(ts.input_path, *"", "{}", ts.input_path);
        assert_eq!(ts.num_repeat, 0, "{}", ts.num_repeat);
        assert_eq!(ts.num_stat, 0, "{}", ts.num_stat);
        assert_eq!(ts.num_read, 0, "{}", ts.num_read);
        assert_eq!(ts.num_read_bytes, 0, "{}", ts.num_read_bytes);
        assert_eq!(ts.num_write, 0, "{}", ts.num_write);
        assert_eq!(ts.num_write_bytes, 0, "{}", ts.num_write_bytes);
    }

    #[test]
    fn test_newwrite() {
        let ts = super::ThreadStat::newwrite();
        assert!(!ts.is_reader, "{}", ts.is_reader);
        assert_eq!(ts.input_path, *"", "{}", ts.input_path);
        assert_eq!(ts.num_repeat, 0, "{}", ts.num_repeat);
        assert_eq!(ts.num_stat, 0, "{}", ts.num_stat);
        assert_eq!(ts.num_read, 0, "{}", ts.num_read);
        assert_eq!(ts.num_read_bytes, 0, "{}", ts.num_read_bytes);
        assert_eq!(ts.num_write, 0, "{}", ts.num_write);
        assert_eq!(ts.num_write_bytes, 0, "{}", ts.num_write_bytes);
    }

    #[test]
    fn test_set_time() {
        let mut ts = super::ThreadStat::newread();
        let d = match ts.time_end.duration_since(ts.time_begin) {
            Ok(v) => v,
            Err(e) => panic!("{e}"),
        };
        assert!(d.is_zero(), "{:?} {:?}", ts.time_begin, ts.time_end);

        let d = match ts.time_begin.duration_since(ts.time_end) {
            Ok(v) => v,
            Err(e) => panic!("{e}"),
        };
        assert!(d.is_zero(), "{:?} {:?}", ts.time_end, ts.time_begin);

        ts.set_time_begin();
        std::thread::sleep(std::time::Duration::from_millis(1));
        ts.set_time_end();

        let d = match ts.time_end.duration_since(ts.time_begin) {
            Ok(v) => v,
            Err(e) => panic!("{e}"),
        };
        assert!(!d.is_zero(), "{:?} {:?}", ts.time_begin, ts.time_end);
        assert_ne!(d.as_millis(), 0, "{:?} {:?}", ts.time_begin, ts.time_end);
        assert_ne!(d.as_micros(), 0, "{:?} {:?}", ts.time_begin, ts.time_end);
        assert_ne!(d.as_nanos(), 0, "{:?} {:?}", ts.time_begin, ts.time_end);

        if let Ok(v) = ts.time_begin.duration_since(ts.time_end) {
            panic!("{v:?}");
        }
    }

    #[test]
    fn test_time_elapsed() {
        let mut ts = super::ThreadStat::newread();
        ts.set_time_begin();

        std::thread::sleep(std::time::Duration::from_millis(100));
        let d = ts.time_elapsed();
        assert!(d.as_millis() >= 100, "{}", d.as_millis());
        assert_eq!(d.as_secs(), 0, "{}", d.as_secs());

        std::thread::sleep(std::time::Duration::from_millis(100));
        let d = ts.time_elapsed();
        assert!(d.as_millis() >= 200, "{}", d.as_millis());
        assert_eq!(d.as_secs(), 0, "{}", d.as_secs());
    }

    #[test]
    fn test_inc_num_repeat() {
        let mut ts = super::ThreadStat::newread();
        ts.inc_num_repeat();
        assert_eq!(ts.num_repeat, 1, "{}", ts.num_repeat);
        ts.inc_num_repeat();
        assert_eq!(ts.num_repeat, 2, "{}", ts.num_repeat);
    }

    #[test]
    fn test_inc_num_stat() {
        let mut ts = super::ThreadStat::newread();
        ts.inc_num_stat();
        assert_eq!(ts.num_stat, 1, "{}", ts.num_stat);
        ts.inc_num_stat();
        assert_eq!(ts.num_stat, 2, "{}", ts.num_stat);
    }

    #[test]
    fn test_inc_num_read() {
        let mut ts = super::ThreadStat::newread();
        ts.inc_num_read();
        assert_eq!(ts.num_read, 1, "{}", ts.num_read);
        ts.inc_num_read();
        assert_eq!(ts.num_read, 2, "{}", ts.num_read);
    }

    #[test]
    fn test_add_num_read_bytes() {
        let mut ts = super::ThreadStat::newread();
        let siz = 1234;
        ts.add_num_read_bytes(siz);
        assert_eq!(ts.num_read_bytes, siz, "{}", ts.num_read);
        ts.add_num_read_bytes(siz);
        assert_eq!(ts.num_read_bytes, siz * 2, "{}", ts.num_read);
        ts.add_num_read_bytes(0);
        assert_eq!(ts.num_read_bytes, siz * 2, "{}", ts.num_read);
    }

    #[test]
    fn test_inc_num_write() {
        let mut ts = super::ThreadStat::newread();
        ts.inc_num_write();
        assert_eq!(ts.num_write, 1, "{}", ts.num_write);
        ts.inc_num_write();
        assert_eq!(ts.num_write, 2, "{}", ts.num_write);
    }

    #[test]
    fn test_add_num_write_bytes() {
        let mut ts = super::ThreadStat::newread();
        let siz = 1234;
        ts.add_num_write_bytes(siz);
        assert_eq!(ts.num_write_bytes, siz, "{}", ts.num_write);
        ts.add_num_write_bytes(siz);
        assert_eq!(ts.num_write_bytes, siz * 2, "{}", ts.num_write);
        ts.add_num_write_bytes(0);
        assert_eq!(ts.num_write_bytes, siz * 2, "{}", ts.num_write);
    }
}
