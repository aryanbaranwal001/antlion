//! One run of a command as a child process, read line by line in the background.

use std::{
    io::{BufRead, BufReader, Read},
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::{Duration, Instant},
};

/// One line of output, and whether it came on stderr.
pub struct Line {
    pub text: String,
    pub err: bool,
}

/// How a run ended.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum End {
    /// The exit code; `None` when a signal stopped the process.
    Exited(Option<i32>),
    Cancelled,
}

pub struct Run {
    /// The arguments, without the program name.
    pub args: Vec<String>,
    pub lines: Vec<Line>,
    pub started: Instant,
    /// When and how the run ended, once it has.
    pub end: Option<(End, Duration)>,
    child: Child,
    rx: Receiver<Line>,
}

impl Run {
    /// Start `program` with `args` in the background. Each line of stdout and stderr is
    /// read on its own thread, so neither pipe can fill up and stop the child.
    pub fn start(program: &str, args: Vec<String>) -> std::io::Result<Run> {
        let mut child = Command::new(program)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let (tx, rx) = mpsc::channel();
        let out = child.stdout.take().expect("stdout is piped");
        let err = child.stderr.take().expect("stderr is piped");
        read_lines(out, false, tx.clone());
        read_lines(err, true, tx);
        Ok(Run {
            args,
            lines: Vec::new(),
            started: Instant::now(),
            end: None,
            child,
            rx,
        })
    }

    /// Take the lines that came since the last call. Once both pipes are closed, wait for
    /// the child and record how it ended.
    pub fn poll(&mut self) {
        loop {
            match self.rx.try_recv() {
                Ok(line) => self.lines.push(line),
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        if self.end.is_none() {
            let code = self.child.wait().ok().and_then(|s| s.code());
            self.end = Some((End::Exited(code), self.started.elapsed()));
        }
    }

    pub fn running(&self) -> bool {
        self.end.is_none()
    }

    /// Stop the child, and wait for it so that no zombie process stays.
    pub fn cancel(&mut self) {
        if self.running() {
            let _ = self.child.kill();
            let _ = self.child.wait();
            self.end = Some((End::Cancelled, self.started.elapsed()));
        }
    }

    #[cfg(test)]
    fn pid(&self) -> u32 {
        self.child.id()
    }
}

/// A run that is dropped, for example when the user quits, stops its child too.
impl Drop for Run {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// Send each line of `pipe` on `tx` until the pipe closes. A line that is not UTF-8 is
/// kept, with the bad bytes replaced.
fn read_lines(pipe: impl Read + Send + 'static, err: bool, tx: mpsc::Sender<Line>) {
    thread::spawn(move || {
        let mut r = BufReader::new(pipe);
        let mut buf = Vec::new();
        while r.read_until(b'\n', &mut buf).is_ok_and(|n| n > 0) {
            if buf.last() == Some(&b'\n') {
                buf.pop();
            }
            let text = String::from_utf8_lossy(&buf).into_owned();
            if tx.send(Line { text, err }).is_err() {
                return;
            }
            buf.clear();
        }
    });
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// Poll `run` until it ends, for at most 5 seconds.
    pub fn finish(run: &mut Run) {
        let stop = Instant::now() + Duration::from_secs(5);
        while run.running() && Instant::now() < stop {
            run.poll();
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn lines_and_exit_code() {
        let script = "echo one; echo two >&2; printf 'three'; exit 3";
        let mut run = Run::start("sh", vec!["-c".into(), script.into()]).unwrap();
        finish(&mut run);
        assert_eq!(run.end.map(|e| e.0), Some(End::Exited(Some(3))));

        let mut out: Vec<(&str, bool)> =
            run.lines.iter().map(|l| (l.text.as_str(), l.err)).collect();
        // The two pipes are read on two threads, so only the order within each is fixed.
        out.sort();
        assert_eq!(out, [("one", false), ("three", false), ("two", true)]);
    }

    #[test]
    fn cancel() {
        let mut run = Run::start("sleep", vec!["30".into()]).unwrap();
        let pid = run.pid();
        run.cancel();
        assert_eq!(run.end.map(|e| e.0), Some(End::Cancelled));
        assert!(run.end.unwrap().1 < Duration::from_secs(5));
        // The child is gone: no process, and no zombie, has its id.
        assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
        run.poll();
        assert_eq!(run.end.map(|e| e.0), Some(End::Cancelled));
    }

    #[test]
    fn a_program_that_does_not_exist() {
        assert!(Run::start("antlion-no-such-program", Vec::new()).is_err());
    }
}
