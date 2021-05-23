#![windows_subsystem = "windows"]

use anyhow::{bail, Result};
use bindings::Windows::Win32::{
    System::Diagnostics::Debug::*, System::SystemServices::*, System::Threading::*,
    System::WindowsProgramming::*,
};
use std::mem;
use std::path::PathBuf;
use std::ptr;
use structopt::StructOpt;
use windows::IntoParam;

macro_rules! windows_bail {
    ($message:expr) => {{
        anyhow::bail!("{}, last error={:?}", $message, GetLastError());
    }};
}

macro_rules! println {
    ($($tokens:tt)*) => {{
        #[cfg(debug_assertions)]
        std::println!($($tokens)*);
    }};
}

#[derive(Debug, StructOpt)]
struct Cli {
    /// Number of CPUs.
    cpus: u32,

    /// Command.
    command: PathBuf,

    /// Working directory (executable's directory by default).
    #[structopt(long, short = "C")]
    working_dir: Option<PathBuf>,
}

impl Cli {
    unsafe fn run(self) -> Result<()> {
        if self.command.is_relative() {
            bail!("The command must be an absolute path, e.g., C:\\Windows\\System32\\calc.exe");
        }

        let affinity = 2_usize.pow(self.cpus) - 1;
        println!(
            "affinity={affinity:#064b} ({affinity})",
            affinity = affinity
        );

        let current_process = GetCurrentProcess();
        if !SetProcessAffinityMask(current_process, affinity).as_bool() {
            windows_bail!("Could not set current process affinity");
        }

        let mut command: windows::Param<'static, PWSTR> =
            self.command.to_string_lossy().to_string().into_param();
        let mut working_dir: windows::Param<'static, PWSTR> = self
            .working_dir
            .as_deref()
            .unwrap_or_else(|| {
                self.command
                    .parent()
                    .expect("the path has been ensured to be absolute; qed")
            })
            .to_string_lossy()
            .to_string()
            .into_param();

        let mut startup_info = STARTUPINFOW::default();
        startup_info.cb = mem::size_of::<STARTUPINFOW>() as u32;
        let mut process_information = PROCESS_INFORMATION::default();
        if !CreateProcessW(
            command.abi(),
            PWSTR::NULL,
            ptr::null_mut(),
            ptr::null_mut(),
            false,
            0.into(),
            ptr::null_mut(),
            working_dir.abi(),
            &mut startup_info,
            &mut process_information,
        )
        .as_bool()
        {
            windows_bail!("Could not create process");
        }

        WaitForSingleObject(process_information.hProcess, INFINITE);

        Ok(())
    }
}

fn main() -> Result<()> {
    unsafe { Cli::from_args().run() }
}
