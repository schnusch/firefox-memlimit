use clap::Arg;
use log::{trace, info, warn, error};
use nix::sys::signal::{sigprocmask, SigSet, SigmaskHow::SIG_BLOCK};
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::{execvp, fork, ForkResult, getpid, getuid, geteuid, seteuid};
use std::ffi::CString;
use std::fs::remove_dir;
use std::io::Write;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

struct TempMemCgroup {
	temp: tempfile::TempDir,
}

impl TempMemCgroup {
	fn new() -> std::io::Result<Self> {
		tempfile::Builder::new()
			.prefix("firefox.")
			.tempdir_in("/sys/fs/cgroup/memory")
			.map(|x| TempMemCgroup { temp: x })
	}

	fn path(&self) -> &Path {
		self.temp.path()
	}
}

impl Drop for TempMemCgroup {
	fn drop(&mut self) {
		let retries = 5;
		for i in 0..retries {
			if i > 0 {
				sleep(Duration::from_secs(1));
			}
			match remove_dir(self.path()) {
				Ok(()) => {
					info!("removed temporary cgroup {:?}", self.path());
					return;
				},
				Err(e) => {
					warn!("cannot remove temporary cgroup {:?}: {} (try {}/{})",
						self.path(), e, i + 1, retries);
				}
			}
		}
	}
}

fn parse_mem(mem: &str) -> Result<u64, std::num::ParseIntError> {
	let shift = match mem.get(mem.len() - 1 ..) {
		Some("K") => 10,
		Some("M") => 20,
		Some("G") => 30,
		Some("T") => 40,
		_ => 0,
	};
	let mem = match shift {
		0 => mem,
		_ => &mem[.. mem.len() - 1],
	};
	match mem.parse::<u64>() {
		Ok(x)  => Ok(x << shift),
		Err(e) => Err(e),
	}
}

fn parse_cmdline_args() -> Result<(u64, Vec<String>), String> {
	let args = clap::App::new("firefox-memlimit")
		.setting(clap::AppSettings::TrailingVarArg)
		.arg(Arg::with_name("mem")
			.short("m")
			.long("memory")
			.help("set firefox's memory limit, default 2G")
			.takes_value(true))
		.arg(Arg::with_name("args")
			.multiple(true)
			.allow_hyphen_values(true))
		.get_matches();

	let mem = args.value_of("mem").unwrap_or("2G");
	let mem = match parse_mem(&mem) {
		Ok(m)  => m,
		Err(e) => return Err(format!("invalid memory limit {:?}: {}", mem, e)),
	};
	let args: Vec<String> = args.values_of("args")
		.unwrap_or_default()
		.map(String::from)
		.collect();

	return Ok((mem, args));
}

fn write_memlimits(tmp: &Path, mem: u64) -> std::io::Result<()> {
	let mem = format!("{}\n", &mem).into_bytes();
	for path in vec!["memory.limit_in_bytes", "memory.memsw.limit_in_bytes"] {
		let path = tmp.join(path);
		std::fs::OpenOptions::new()
			.write(true)
			.open(&path)?
			.write_all(&mem)?;
	}
	Ok(())
}

fn enter_cgroup(tmp: &Path) -> std::io::Result<()> {
	let pid = format!("{}\n", getpid()).into_bytes();
	std::fs::OpenOptions::new()
		.write(true)
		.open(tmp.join("cgroup.procs"))?
		.write_all(&pid)
}

fn actual_main(mem: u64, args: Vec<String>) -> Result<i32, String> {
	let tmp = match TempMemCgroup::new() {
		Ok(d)  => d,
		Err(e) => return Err(format!("cannot create cgroup: {}", e)),
	};
	info!("created temporary cgroup {:?}", tmp.path());

	if let Err(e) = write_memlimits(tmp.path(), mem) {
		return Err(format!("cannot write memory limits for cgroup {:?}: {}", tmp.path(), e));
	}
	trace!("limiting cgroup's memory to {} bytes", mem);

	let child_pid = unsafe { fork() };
	let child_pid = match child_pid {
		Ok(ForkResult::Parent { child, .. }) => child,
		Ok(ForkResult::Child) => {
			if let Err(e) = enter_cgroup(tmp.path()) {
				error!("cannot enter cgroup: {}", e);
				std::process::exit(1);
			}
			trace!("added child process ({}) to {:?}", getpid(), tmp.path());

			trace!("dropping effective UID form {} to {}...", geteuid(), getuid());
			if let Err(e) = seteuid(getuid()) {
				error!("cannot set effective user-id to {}: {}", getuid(), e);
				std::process::exit(1);
			}

			let args: Vec<CString> =
				std::iter::once(String::from("firefox"))
					.chain(args.into_iter())
					.map(|x| CString::new(x).unwrap())
					.collect();
			match execvp(&args[0], &args) {
				Ok(_)  => unreachable!(),
				Err(e) => {
					error!("cannot start firefox: {}", e);
					std::process::exit(1);
				}
			}
		},
		Err(e) => {
			return Err(format!("cannot create process: {}", e));
		},
	};

	match sigprocmask(SIG_BLOCK, Some(&SigSet::all()), None) {
		Ok(()) => trace!("ignoring all signals"),
		Err(e) => warn!("cannot set signal mask: {}", e),
	}

	info!("waiting for child process ({}) to exit...", child_pid);
	loop {
		match waitpid(child_pid, None) {
			Err(e) => {
				return Err(format!("cannot wait for firefox: {}", e));
			},
			Ok(WaitStatus::Exited(_pid, exitcode)) => {
				return Ok(exitcode);
			},
			Ok(WaitStatus::Signaled(_pid, signal, _)) => {
				return Ok(128 | (signal as i32));
			},
			_ => {}
		}
	}
}

fn main() {
	env_logger::Builder::from_default_env()
		.filter_level(log::LevelFilter::Trace)
		.init();

	let (mem, args) = match parse_cmdline_args() {
		Ok(x)  => x,
		Err(e) => {
			error!("cannot parse command line: {}", e);
			std::process::exit(2);
		}
	};

	match actual_main(mem, args) {
		Ok(exitcode) => {
			trace!("exiting with {}", exitcode);
			std::process::exit(exitcode);
		},
		Err(e) => {
			error!("{}", e);
			std::process::exit(1);
		},
	}
}
