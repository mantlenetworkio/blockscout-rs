// SPDX-License-Identifier: LicenseRef-Blockscout

#![allow(clippy::all)]
#![allow(unused_imports)]
pub mod blockscout {
    pub mod stats {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/blockscout.stats.v1.rs"));
        }
    }
}

#[cfg(test)]
mod tests;
