// SPDX-License-Identifier: GPL-2.0

//! Types for working with the block layer

pub mod bio;
pub mod mq;

pub use crate::bindings::{req_op, req_op_REQ_OP_READ, req_op_REQ_OP_WRITE};
