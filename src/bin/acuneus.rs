fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = env_logger::try_init();

    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        eprintln!("usage: acuneus [shader]");
        eprintln!("example: acuneus roto");
        return Ok(());
    }

    let bin_name = args.first().cloned().unwrap_or_else(|| "roto".to_string());
    let bin_name = bin_name.strip_prefix("cuneus-").unwrap_or(&bin_name);
    let mode = option_env!("ACUNEUS_RUNNER_CONTENT").unwrap_or("both");

    match mode {
        "examples" => acuneus::embedded::run_bin(bin_name),
        "bins" => run_bin_executable(bin_name),
        "both" => acuneus::embedded::run_bin(bin_name).or_else(|_| run_bin_executable(bin_name)),
        _ => acuneus::embedded::run_bin(bin_name).or_else(|_| run_bin_executable(bin_name)),
    }
}

fn run_bin_executable(bin_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let current_exe = std::env::current_exe()?;
    let executable_dir = current_exe
        .parent()
        .ok_or("acuneus executable has no parent directory")?;
    let mut executable = executable_dir.join(format!("cuneus-{bin_name}"));
    if bin_name == "roto" {
        executable = executable_dir.join("roto");
    }
    if cfg!(windows) {
        executable.set_extension("exe");
    }

    let mut command = std::process::Command::new(&executable);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let status = command
        .envs(std::env::vars().filter(|(key, _)| key.starts_with("CUNEUS_")))
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{} exited with {status}", executable.display()).into())
    }
}
