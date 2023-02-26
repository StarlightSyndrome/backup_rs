# backup_rs

This is just a simple wrapper to run rsync with versioned and hardlinked target directories.

Written in Rust.

The next version uses tokio to thread out a running rsync process and process its output to filter and display information.  