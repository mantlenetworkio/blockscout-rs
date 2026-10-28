// SPDX-License-Identifier: LicenseRef-Blockscout

#![allow(clippy::all)]
pub mod blockscout {
    pub mod tac_operation_lifecycle {
        pub mod v1 {
            include!(concat!(
                env!("OUT_DIR"),
                "/blockscout.tac_operation_lifecycle.v1.rs"
            ));
        }
    }
}
