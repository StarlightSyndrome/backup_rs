#![feature(iterator_try_collect)]

use std::{
    process::{Stdio, ExitCode, ExitStatus},
    fs::{read_dir, DirEntry},
    path::{Path, },
    str,
};


use anyhow::{Result, ensure, format_err};
use chrono::prelude::*;
use tokio::io::{BufReader, AsyncBufReadExt};
use tokio::process::Command;




use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, about, long_about = "Run backups based on rsync")]
struct Cli {
    #[arg(short, long, help="The directory to backup from")]
    source_dir: String,
    #[arg(short, long, help="The directory to back to")]
    target_dir: String,
    #[arg(short='V', long, help="Create versioned backup.")]
    versioned: bool,
    #[arg(short, long, help="Exclude **Cache**")]
    no_exclude_caches: bool,
    #[arg(short='E', long, help="Override exclude patterns, list of patterns separated by comma")]
    exclude_override: Option<String>,
    #[arg(short, long, help="Pass these args through to rsync")]
    pass_args: Option<String>,
}



async fn run_rsync(cli: &mut Cli) -> Result<ExitStatus> {
    
    let mut args: Vec<&str> = vec!["-axP", "--stats"];
    
    let mut other_args: Vec<String> = Vec::new();
    if cli.versioned {
        make_versioned_dir(cli, &mut other_args)?;
        args.extend(other_args.iter().map(|x|x.as_str()));
    }
    if ! cli.no_exclude_caches {
        args.extend(["--exclude", "**Cache**", "--exclude", "**cache**"]);
    }

    if let Some(exclude_override) = cli.exclude_override.as_deref() {
        for excl in exclude_override.split(',') {
            args.extend(["--exclude", excl]);
        }
    }

    if let Some(pass_args) = cli.pass_args.as_deref() {
        args.extend(pass_args.split(' '));
    }

    args.push(&cli.source_dir);
    args.push(&cli.target_dir);

    println!("Running rsync {}", args.join(" "));

    let mut child = Command::new("rsync")
        .args(args.as_slice())
        .stdout(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take()
        .expect("cannot read stdout");

    // rsync progress prints and updates state in one line using carriage return 0x0d
    // so we split on CR first. Look for newline later
    let mut reader = BufReader::new(stdout)
        .split(0x0d);
    
    let exit_status = tokio::spawn(async move {
        let status = child.wait().await
            .expect("process did not return status");

        println!("child exit code was: {}", status.code().unwrap_or_default());
        
        status
    });

    
    while let Some(segment) = reader.next_segment().await? {
        if segment.is_empty() {
            break;
        }
        let parts = segment
            .split(|x| *x==0x0au8)
            .collect::<Vec<&[u8]>>();

        if parts.len() == 1 {
            println!("Progress: {}", std::str::from_utf8(parts[0])?);
        } else {
            for p in parts {
                let line = std::str::from_utf8(p)?;
                println!("Line: {}", line);
            }
        }
    }

    Ok(exit_status.await.unwrap())
}

fn dirname_is_valid_date(dir_entry: DirEntry) -> Result<String> {
    let dirname = dir_entry.file_name().into_string().unwrap();

    ensure!(dir_entry.file_type().unwrap().is_dir(), 
        format_err!("Directory entry {dirname} is not a directory"));
    ensure!(Local.datetime_from_str(dirname.as_str(), "%Y%m%d%H%M").is_ok(), 
        format_err!("Directory entry name {dirname} is not in datetime format of YYYYmmddHHMM"));
    
    Ok(dirname)
}

fn make_versioned_dir(cli: &mut Cli, other_args: &mut Vec<String>) -> Result<()>   {
    // get latest backup in target dir
    // versioned dirs are datetime: YYYYmmddhhmm
    let mut dirs: Vec<String> = read_dir(&cli.target_dir)?
        .map(|d| dirname_is_valid_date(d?))
        .try_collect::<_>()?;
        
   
    //build target dir from date and time for versioned backups 
    let now = Local::now();
    let now_dir = format!("{}", now.format("%Y%m%d%H%M"));
    
    let target_dir = cli.target_dir.to_owned();
    cli.target_dir = Path::new(&target_dir)
        .join(now_dir).to_string_lossy().to_string();

    if dirs.len() > 1 {
        dirs.sort();
        other_args.push(
            format!("--link-dest={}", Path::new(&target_dir)
            .join(dirs.pop().unwrap())
            .to_str().unwrap())
        );
    }
    Ok(())
}


#[tokio::main]
async fn main() -> Result<ExitCode> {
    let mut cli = Cli::parse();

    let status = run_rsync(&mut cli).await;

    match status {
        Ok(exit_status) => Ok(ExitCode::from(exit_status.code().unwrap() as u8)),
        Err(e) => Err(e)
    }
}