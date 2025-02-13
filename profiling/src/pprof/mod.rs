// Unless explicitly stated otherwise all files in this repository are licensed under the Apache License Version 2.0.
// This product includes software developed at Datadog (https://www.datadoghq.com/). Copyright 2023-Present Datadog, Inc.

mod proto;

pub mod sliced_proto;
pub use proto::*;

#[cfg(test)]
pub mod test_utils;
#[cfg(test)]
pub use test_utils::*;
