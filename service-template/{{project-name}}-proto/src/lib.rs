// SPDX-License-Identifier: LicenseRef-Blockscout

#![allow(clippy::all)]
pub mod blockscout {
    pub mod {{crate_name}} {
        pub mod v1 {
            include!(concat!(
                env!("OUT_DIR"),
                "/blockscout.{{crate_name}}.v1.rs"
            ));
        }
    }
}
