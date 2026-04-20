use crate::util::print_status;
use anyhow::{bail, Context, Result};
use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

pub(crate) struct GameWrapper {
	exe_path: PathBuf,
	args: Vec<String>,
}

impl GameWrapper {
	pub(crate) fn new(exe_path: PathBuf, args: Vec<String>) -> Self {
		Self { exe_path, args }
	}

	fn stream_process_output<R: Read + Send + 'static>(
		reader: R,
		tx: mpsc::Sender<Vec<u8>>,
	) -> thread::JoinHandle<()> {
		thread::spawn(move || {
			let mut reader = BufReader::new(reader);
			loop {
				let mut bytes = Vec::new();
				match reader.read_until(b'\n', &mut bytes) {
					Ok(0) => break,
					Ok(_) => {
						if tx.send(bytes).is_err() {
							break;
						}
					}
					Err(_) => break,
				}
			}
		})
	}

	pub(crate) fn run(
		self,
		ctrl_c_flag: Arc<AtomicBool>,
		mut on_output: impl FnMut(&[u8]),
	) -> Result<ExitStatus> {
		let mut command = Command::new(&self.exe_path);
		command
			.args(&self.args)
			.stdout(Stdio::piped())
			.stderr(Stdio::piped());

		let mut child = command
			.spawn()
			.with_context(|| format!("failed to launch executable: {}", self.exe_path.display()))?;

		let stdout = child
			.stdout
			.take()
			.context("failed to capture child stdout")?;
		let stderr = child
			.stderr
			.take()
			.context("failed to capture child stderr")?;

		let (tx, rx) = mpsc::channel();
		let stdout_thread = Self::stream_process_output(stdout, tx.clone());
		let stderr_thread = Self::stream_process_output(stderr, tx);

		ctrl_c_flag.store(false, Ordering::SeqCst);

		let mut process_status = None;
		let mut streams_open = true;
		let mut sent_stop = false;

		loop {
			if ctrl_c_flag.load(Ordering::SeqCst) && !sent_stop {
				sent_stop = true;
				print_status("Stopping", "Ctrl+C received, terminating process");
				let _ = child.kill();
			}

			if streams_open {
				match rx.recv_timeout(Duration::from_millis(100)) {
					Ok(bytes) => on_output(&bytes),
					Err(mpsc::RecvTimeoutError::Timeout) => {}
					Err(mpsc::RecvTimeoutError::Disconnected) => streams_open = false,
				}
			}

			if process_status.is_none() {
				if let Some(status) = child.try_wait().context("failed to query process status")? {
					process_status = Some(status);
				}
			}

			if process_status.is_some() && !streams_open {
				break;
			}
		}

		let _ = stdout_thread.join();
		let _ = stderr_thread.join();

		let status = match process_status {
			Some(status) => status,
			None => child.wait().context("failed to wait for process exit")?,
		};

		if !status.success() {
			bail!("process exited with status {status}");
		}

		Ok(status)
	}
}
