use std::{
    env,
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{ChildStdout, Command, Stdio},
};

use colored::Colorize;

fn xcodebuild_list(workspace: &str) -> Result<BufReader<ChildStdout>, Box<dyn std::error::Error>> {
    let mut child = Command::new("xcodebuild")
        .arg("-quiet")
        .arg("-workspace")
        .arg(workspace)
        .arg("-list")
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn xcodebuild list process");

    let stdout = child.stdout.take().expect("Failed to grab stdout");
    Ok(BufReader::new(stdout))
}

fn xcodebuild_run(
    workspace: &str,
    scheme: &str,
    buildlog_dir: &Path,
) -> Result<bool, Box<dyn std::error::Error>> {
    let buildlog = buildlog_dir.join(format!("{}.log", scheme));
    let mut logfile = File::create(buildlog)?;

    let builderr = buildlog_dir.join(format!("{}.err.log", scheme));
    let errfile = File::create(builderr)?;

    let mut child = Command::new("xcodebuild")
        .arg("-workspace")
        .arg(workspace)
        .arg("-scheme")
        .arg(scheme)
        .stdout(Stdio::piped())
        .stderr(Stdio::from(errfile))
        .spawn()
        .expect("Failed to spawn xcodebuild list process");

    let stdout = child.stdout.take().expect("Failed to grab stdout");
    let reader = BufReader::new(stdout);

    let mut build_status = false;
    for line in reader.lines() {
        let l = line?;
        logfile.write_all(&l.as_bytes())?;
        logfile.write_all("\n".as_bytes())?;

        if l.starts_with("** BUILD SUCCEEDED **") {
            build_status = true;
        }
    }

    Ok(build_status)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let Some(workspace_path) = args.get(1) else {
        return Err("Missing workspace argument".into());
    };
    let patterns: Option<Vec<&str>> = args.get(2).map(|p| p.split(",").collect());

    let reader = xcodebuild_list(workspace_path)?;
    let schemes: Vec<String> = reader
        .lines()
        .filter_map(|l| l.ok())
        .filter(|l| l.starts_with(&" ".repeat(8)))
        .map(|l| l.trim().to_string())
        .filter(|s| {
            if let Some(ref ps) = patterns {
                ps.iter().any(|p| s.starts_with(p))
            } else {
                true
            }
        })
        .collect();

    let buildlog_dir = Path::new("buildlogs");
    if !buildlog_dir.exists() {
        fs::create_dir("buildlogs")?;
    }

    for s in &schemes {
        let build_status = xcodebuild_run(workspace_path, s, buildlog_dir)?;
        if build_status {
            println!("{}", s.green());
        } else {
            println!("{}", s.red());
        }
    }

    Ok(())
}
