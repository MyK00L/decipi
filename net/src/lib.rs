#![feature(lazy_cell)]
#![feature(ip_bits)]
#![allow(dead_code)]
mod connection;
pub mod message;
mod socket;

#[cfg(test)]
mod tests {}
