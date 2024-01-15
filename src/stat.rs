#[derive(Clone, Debug)]
pub struct ThreadStat {
    pub is_reader: bool,
    input_path: String,
    time_begin: std::time::SystemTime,
    time_end: std::time::SystemTime,
    num_repeat: usize,
    num_stat: usize,
    num_read: usize,
    num_read_bytes: usize,
    num_write: usize,
    num_write_bytes: usize,
}

impl Default for ThreadStat {
    fn default() -> ThreadStat {
        ThreadStat {
            is_reader: true,
            input_path: "".to_string(),
            time_begin: std::time::UNIX_EPOCH,
            time_end: std::time::UNIX_EPOCH,
            num_repeat: 0,
            num_stat: 0,
            num_read: 0,
            num_read_bytes: 0,
            num_write: 0,
            num_write_bytes: 0,
        }
    }
}

pub fn newread() -> ThreadStat {
    ThreadStat {
        is_reader: true,
        ..Default::default()
    }
}

pub fn newwrite() -> ThreadStat {
    ThreadStat {
        is_reader: false,
        ..Default::default()
    }
}

impl ThreadStat {
    pub fn set_input_path(&mut self, f: &str) {
        self.input_path = f.to_string();
    }

    pub fn set_time_begin(&mut self) {
        self.time_begin = std::time::SystemTime::now();
    }

    pub fn set_time_end(&mut self) {
        self.time_end = std::time::SystemTime::now();
    }

    pub fn time_elapsed(&self) -> u64 {
        self.time_begin.elapsed().unwrap().as_secs()
    }

    pub fn inc_num_repeat(&mut self) {
        self.num_repeat += 1;
    }

    pub fn inc_num_stat(&mut self) {
        self.num_stat += 1;
    }

    pub fn inc_num_read(&mut self) {
        self.num_read += 1;
    }

    pub fn add_num_read_bytes(&mut self, siz: usize) {
        self.num_read_bytes += siz;
    }

    pub fn inc_num_write(&mut self) {
        self.num_write += 1;
    }

    pub fn add_num_write_bytes(&mut self, siz: usize) {
        self.num_write_bytes += siz;
    }
}

pub fn print_stat(tsv: &Vec<ThreadStat>) {
    // repeat
    let mut width_repeat = "repeat".len();
    for ts in tsv.iter() {
        let s = ts.num_repeat.to_string();
        if s.len() > width_repeat {
            width_repeat = s.len();
        }
    }

    // stat
    let mut width_stat = "stat".len();
    for ts in tsv.iter() {
        let s = ts.num_stat.to_string();
        if s.len() > width_stat {
            width_stat = s.len();
        }
    }

    // read
    let mut width_read = "read".len();
    for ts in tsv.iter() {
        let s = ts.num_read.to_string();
        if s.len() > width_read {
            width_read = s.len();
        }
    }

    // read[B]
    let mut width_read_bytes = "read[B]".len();
    for ts in tsv.iter() {
        let s = ts.num_read_bytes.to_string();
        if s.len() > width_read_bytes {
            width_read_bytes = s.len();
        }
    }

    // write
    let mut width_write = "write".len();
    for ts in tsv.iter() {
        let s = ts.num_write.to_string();
        if s.len() > width_write {
            width_write = s.len();
        }
    }

    // write[B]
    let mut width_write_bytes = "write[B]".len();
    for ts in tsv.iter() {
        let s = ts.num_write_bytes.to_string();
        if s.len() > width_write_bytes {
            width_write_bytes = s.len();
        }
    }

    // sec
    let mut width_sec = "sec".len();
    let mut num_sec = vec![0 as f64; tsv.len()];
    for (i, t) in num_sec.iter_mut().enumerate() {
        let sec = tsv[i]
            .time_end
            .duration_since(tsv[i].time_begin)
            .unwrap()
            .as_secs_f64();
        *t = f64::trunc(sec * 100.0) / 100.0; // cut decimals
    }
    for t in num_sec.iter() {
        let s = t.to_string();
        if s.len() > width_sec {
            width_sec = s.len();
        }
    }

    // MiB/sec
    let mut width_mibs = "MiB/sec".len();
    let mut num_mibs = vec![0 as f64; tsv.len()];
    for (i, x) in num_mibs.iter_mut().enumerate() {
        let mib = (tsv[i].num_read_bytes + tsv[i].num_write_bytes) as f64 / (1 << 20) as f64;
        let mibs = mib / num_sec[i];
        *x = f64::trunc(mibs * 100.0) / 100.0; // cut decimals
    }
    for x in num_mibs.iter() {
        let s = x.to_string();
        if s.len() > width_mibs {
            width_mibs = s.len();
        }
    }

    // path
    let mut width_path = "path".len();
    for ts in tsv.iter() {
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
        print!("{}", s);
        slen += s.len();
        if lw[i] > s.len() {
            print!("{}", " ".repeat(lw[i] - s.len()));
            slen += lw[i] - s.len();
        }
        if i != ls.len() - 1 {
            print!(" ");
            slen += 1;
        }
    }
    println!();
    println!("{}", "-".repeat(slen));

    for i in 0..nlines {
        // index
        print!("#");
        let s = i.to_string();
        print!("{} ", s); // left align
        if width_index > s.len() {
            print!("{}", " ".repeat(width_index - s.len()));
        }

        // type
        if tsv[i].is_reader {
            print!("reader ");
        } else {
            print!("writer ");
        }

        // repeat
        let s = tsv[i].num_repeat.to_string();
        let w = lw[0];
        if w > s.len() {
            print!("{}", " ".repeat(w - s.len()));
        }
        print!("{} ", s);

        // stat
        let s = tsv[i].num_stat.to_string();
        let w = lw[1];
        if w > s.len() {
            print!("{}", " ".repeat(w - s.len()));
        }
        print!("{} ", s);

        // read
        let s = tsv[i].num_read.to_string();
        let w = lw[2];
        if w > s.len() {
            print!("{}", " ".repeat(w - s.len()));
        }
        print!("{} ", s);

        // read[B]
        let s = tsv[i].num_read_bytes.to_string();
        let w = lw[3];
        if w > s.len() {
            print!("{}", " ".repeat(w - s.len()));
        }
        print!("{} ", s);

        // write
        let s = tsv[i].num_write.to_string();
        let w = lw[4];
        if w > s.len() {
            print!("{}", " ".repeat(w - s.len()));
        }
        print!("{} ", s);

        // write[B]
        let s = tsv[i].num_write_bytes.to_string();
        let w = lw[5];
        if w > s.len() {
            print!("{}", " ".repeat(w - s.len()));
        }
        print!("{} ", s);

        // sec
        let s = num_sec[i].to_string();
        let w = lw[6];
        if w > s.len() {
            print!("{}", " ".repeat(w - s.len()));
        }
        print!("{} ", s);

        // MiB/sec
        let s = num_mibs[i].to_string();
        let w = lw[7];
        if w > s.len() {
            print!("{}", " ".repeat(w - s.len()));
        }
        print!("{} ", s);

        // path
        let s = &tsv[i].input_path;
        let w = lw[8];
        print!("{} ", s); // left align
        if w > s.len() {
            print!("{}", " ".repeat(w - s.len()));
        }

        println!();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_newread() {
        let ts = super::newread();
        if !ts.is_reader {
            panic!("{}", ts.is_reader);
        }
        if ts.input_path != *"" {
            panic!("{}", ts.input_path);
        }
        if ts.num_repeat != 0 {
            panic!("{}", ts.num_repeat);
        }
        if ts.num_stat != 0 {
            panic!("{}", ts.num_stat);
        }
        if ts.num_read != 0 {
            panic!("{}", ts.num_read);
        }
        if ts.num_read_bytes != 0 {
            panic!("{}", ts.num_read_bytes);
        }
        if ts.num_write != 0 {
            panic!("{}", ts.num_write);
        }
        if ts.num_write_bytes != 0 {
            panic!("{}", ts.num_write_bytes);
        }
    }

    #[test]
    fn test_newwrite() {
        let ts = super::newwrite();
        if ts.is_reader {
            panic!("{}", ts.is_reader);
        }
        if ts.input_path != *"" {
            panic!("{}", ts.input_path);
        }
        if ts.num_repeat != 0 {
            panic!("{}", ts.num_repeat);
        }
        if ts.num_stat != 0 {
            panic!("{}", ts.num_stat);
        }
        if ts.num_read != 0 {
            panic!("{}", ts.num_read);
        }
        if ts.num_read_bytes != 0 {
            panic!("{}", ts.num_read_bytes);
        }
        if ts.num_write != 0 {
            panic!("{}", ts.num_write);
        }
        if ts.num_write_bytes != 0 {
            panic!("{}", ts.num_write_bytes);
        }
    }

    #[test]
    fn test_set_time() {
        let mut ts = super::newread();
        let d = match ts.time_end.duration_since(ts.time_begin) {
            Ok(v) => v,
            Err(e) => panic!("{}", e),
        };
        if !d.is_zero() {
            panic!("{:?} {:?}", ts.time_begin, ts.time_end);
        }

        let d = match ts.time_begin.duration_since(ts.time_end) {
            Ok(v) => v,
            Err(e) => panic!("{}", e),
        };
        if !d.is_zero() {
            panic!("{:?} {:?}", ts.time_end, ts.time_begin);
        }

        ts.set_time_begin();
        std::thread::sleep(std::time::Duration::from_millis(1));
        ts.set_time_end();

        let d = match ts.time_end.duration_since(ts.time_begin) {
            Ok(v) => v,
            Err(e) => panic!("{}", e),
        };
        if d.is_zero() {
            panic!("{:?} {:?}", ts.time_begin, ts.time_end);
        }
        if d.as_millis() == 0 {
            panic!("{:?} {:?}", ts.time_begin, ts.time_end);
        }
        if d.as_micros() == 0 {
            panic!("{:?} {:?}", ts.time_begin, ts.time_end);
        }
        if d.as_nanos() == 0 {
            panic!("{:?} {:?}", ts.time_begin, ts.time_end);
        }

        if let Ok(v) = ts.time_begin.duration_since(ts.time_end) {
            panic!("{:?}", v);
        }
    }

    #[test]
    fn test_inc_num_repeat() {
        let mut ts = super::newread();
        ts.inc_num_repeat();
        if ts.num_repeat != 1 {
            panic!("{}", ts.num_repeat);
        }
        ts.inc_num_repeat();
        if ts.num_repeat != 2 {
            panic!("{}", ts.num_repeat);
        }
    }

    #[test]
    fn test_inc_num_stat() {
        let mut ts = super::newread();
        ts.inc_num_stat();
        if ts.num_stat != 1 {
            panic!("{}", ts.num_stat);
        }
        ts.inc_num_stat();
        if ts.num_stat != 2 {
            panic!("{}", ts.num_stat);
        }
    }

    #[test]
    fn test_inc_num_read() {
        let mut ts = super::newread();
        ts.inc_num_read();
        if ts.num_read != 1 {
            panic!("{}", ts.num_read);
        }
        ts.inc_num_read();
        if ts.num_read != 2 {
            panic!("{}", ts.num_read);
        }
    }

    #[test]
    fn test_add_num_read_bytes() {
        let mut ts = super::newread();
        let siz = 1234;
        ts.add_num_read_bytes(siz);
        if ts.num_read_bytes != siz {
            panic!("{}", ts.num_read);
        }
        ts.add_num_read_bytes(siz);
        if ts.num_read_bytes != siz * 2 {
            panic!("{}", ts.num_read);
        }
        ts.add_num_read_bytes(0);
        if ts.num_read_bytes != siz * 2 {
            panic!("{}", ts.num_read);
        }
    }

    #[test]
    fn test_inc_num_write() {
        let mut ts = super::newread();
        ts.inc_num_write();
        if ts.num_write != 1 {
            panic!("{}", ts.num_write);
        }
        ts.inc_num_write();
        if ts.num_write != 2 {
            panic!("{}", ts.num_write);
        }
    }

    #[test]
    fn test_add_num_write_bytes() {
        let mut ts = super::newread();
        let siz = 1234;
        ts.add_num_write_bytes(siz);
        if ts.num_write_bytes != siz {
            panic!("{}", ts.num_write);
        }
        ts.add_num_write_bytes(siz);
        if ts.num_write_bytes != siz * 2 {
            panic!("{}", ts.num_write);
        }
        ts.add_num_write_bytes(0);
        if ts.num_write_bytes != siz * 2 {
            panic!("{}", ts.num_write);
        }
    }
}
