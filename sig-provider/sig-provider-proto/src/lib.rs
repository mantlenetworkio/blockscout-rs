// SPDX-License-Identifier: LicenseRef-Blockscout

#![allow(clippy::all)]
pub mod blockscout {
    pub mod sig_provider {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/blockscout.sig_provider.v1.rs"));
        }
    }
}
