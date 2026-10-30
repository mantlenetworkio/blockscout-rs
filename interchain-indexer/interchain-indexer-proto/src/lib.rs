// SPDX-License-Identifier: LicenseRef-Blockscout

#![allow(clippy::all)]
pub mod blockscout {
    pub mod interchain_indexer {
        pub mod v1 {
            include!(concat!(
                env!("OUT_DIR"),
                "/blockscout.interchain_indexer.v1.rs"
            ));
        }
    }
}
