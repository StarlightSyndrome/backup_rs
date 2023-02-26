use std::{
    process::{Command, Stdio},
    fs::{read_dir, DirEntry},
    path::MAIN_SEPARATOR, 
};
use anyhow::{Result, ensure, format_err};
use chrono::prelude::*;


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



fn run_rsync(cli: &mut Cli) -> Result<()> {
    
    let mut args: Vec<&str> = vec!["-ax", "--stats"];
    
    let mut other_args: Vec<String> = Vec::new();
    if cli.versioned {
        prepare_versioning(cli, &mut other_args)?;
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


    let child = Command::new("rsync")
        .args(args.as_slice())
        .stdout(Stdio::piped())
        .spawn()?;

    let  output = child.wait_with_output().expect("no output");


    println!("{}", std::str::from_utf8(output.stdout.as_slice()).unwrap());
    //match child.code() {
    //    Some(code) => println!("Exited with exit code {}", code),
    //    None => println!("Process terminated by signal"),
    //}

    Ok(())
}

fn dirname_is_valid_date(read_dir: std::io::Result<DirEntry>) -> Result<String> {
    let dir_entry = read_dir.expect("Reading target dir failed");
    let dirname = dir_entry.file_name().into_string().unwrap();

    ensure!(dir_entry.file_type().unwrap().is_dir(), 
        format_err!("Directory entry {dirname} is not a directory"));
    ensure!(Local.datetime_from_str(dirname.as_str(), "%Y%m%d%H%M").is_ok(), 
        format_err!("Directory entry name {dirname} is not in datetime format of YYYYmmddHHMM"));
    
    Ok(dirname)
}

fn prepare_versioning(cli: &mut Cli, other_args: &mut Vec<String>) -> Result<()>   {
    // get latest backup in target dir
    // versioned dirs are datetime: YYYYmmddhhmm
    let mut dirs: Vec<String> = read_dir(&cli.target_dir)
        .expect("cannot list dir")
        .map(|d| dirname_is_valid_date(d).unwrap())
        //.filter_map(|d| d.ok())
        //.filter_map(|x|is_valid_date(&x).ok())
        //.filter_map(|x|x.file_name().into_string().ok())
        .collect();
   
    //build target dir from date and time for versioned backups 
    let now = Local::now();
    let now_dir = format!("{}", now.format("%Y%m%d%H%M"));

    
    cli.target_dir.push(MAIN_SEPARATOR);
    cli.target_dir.push_str(&now_dir);

    let link_dest: String;
    if dirs.len() > 1 {
        dirs.sort();
        link_dest = format!("--link-dest={}", dirs.pop().unwrap());
        other_args.push(link_dest);
    }
    Ok(())
}


fn main() -> Result<()>  {
    let mut cli = Cli::parse();
    run_rsync(&mut cli)
}
