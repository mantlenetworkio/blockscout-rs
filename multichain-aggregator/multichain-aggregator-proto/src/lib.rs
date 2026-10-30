// SPDX-License-Identifier: LicenseRef-Blockscout

#![allow(clippy::all)]
pub mod blockscout {
    pub mod multichain_aggregator {
        pub mod v1 {
            include!(concat!(
                env!("OUT_DIR"),
                "/blockscout.multichain_aggregator.v1.rs"
            ));
        }
    }

    pub mod cluster_explorer {
        pub mod v1 {
            include!(concat!(
                env!("OUT_DIR"),
                "/blockscout.cluster_explorer.v1.rs"
            ));
        }
    }
}
