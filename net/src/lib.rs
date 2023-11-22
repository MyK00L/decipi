#![feature(lazy_cell)]
#![allow(unused)]
#![feature(ip_bits)]
mod connection;
mod init;
pub mod message;
mod socket;

#[cfg(test)]
mod tests {}
