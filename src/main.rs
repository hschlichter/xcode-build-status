use std::{
    env,
    fs::{self, File},
    io::{self, BufRead, BufReader, Write},
    path::Path,
    process::{ChildStdout, Command, Stdio},
    time::Instant,
};

use colored::Colorize;

fn xcodebuild_list(workspace: &str) -> Result<BufReader<ChildStdout>, Box<dyn std::error::Error>> {
    let mut child = Command::new("xcodebuild")
        .arg("-quiet")
        .arg("-workspace")
        .arg(workspace)
        .arg("-list")
        .stdout(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().expect("Failed to grab stdout");
    Ok(BufReader::new(stdout))
}

fn xcodebuild_build(
    workspace: &str,
    scheme: &str,
    buildlog_dir: &Path,
) -> Result<bool, Box<dyn std::error::Error>> {
    let logfile_path = buildlog_dir.join(format!("{}.log", scheme));
    let logfile = File::create(logfile_path)?;

    let errfile_path = buildlog_dir.join(format!("{}.err.log", scheme));
    let errfile = File::create(errfile_path)?;

    let mut xcodebuild = Command::new("xcodebuild")
        .arg("build")
        .arg("-workspace")
        .arg(workspace)
        .arg("-scheme")
        .arg(scheme)
        .stdout(Stdio::piped())
        .stderr(Stdio::from(errfile))
        .spawn()?;

    let mut xcpretty = Command::new("xcpretty")
        .arg("-r")
        .arg("json-compilation-database")
        .arg("-o")
        .arg(format!(
            "{}/{}_compile_commands.json",
            buildlog_dir.to_str().ok_or("No buildlog dir")?,
            scheme
        ))
        .stdin(
            xcodebuild
                .stdout
                .take()
                .expect("Failed to capture stdin from xcodebuild"),
        )
        .stdout(Stdio::from(logfile))
        .spawn()?;

    let status = xcodebuild.wait()?;
    let _ = xcpretty.wait()?;
    Ok(status.success())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let startime = Instant::now();

    let args: Vec<String> = env::args().collect();
    let Some(workspace_path) = args.get(1) else {
        return Err("Missing workspace argument".into());
    };
    let patterns: Option<Vec<&str>> = args.get(2).map(|p| p.split(",").collect());

    println!("Listing workspace");
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

    println!("Building schemes");
    for s in &schemes {
        print!("{}", s);
        io::stdout().flush()?;

        let build_status = xcodebuild_build(workspace_path, s, buildlog_dir)?;
        if build_status {
            println!("\r{}", s.green());
        } else {
            println!("\r{}", s.red());
        }
    }

    let duration = startime.elapsed();
    println!("Time elapsed: {:?}", duration);

    Ok(())
}
