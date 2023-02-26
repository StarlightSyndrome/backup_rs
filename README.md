# backup_rs

This is a simple wrapper to run rsync with versioned and hardlinked target directories.

Written in Rust. Idea is to implement some patterns to be reused with other CLI programs.

Current features of the wrapper:

 - use concurrency to monitor output
 - propagate errors to the main process
 - provide exit code by either the wrapped process or the surrounding code 
