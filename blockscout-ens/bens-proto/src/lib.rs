// SPDX-License-Identifier: LicenseRef-Blockscout

#![allow(clippy::all)]
pub mod blockscout {
    pub mod bens {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/blockscout.bens.v1.rs"));
        }
    }
}
