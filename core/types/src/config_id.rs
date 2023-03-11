// Copyright (c) 2022-2023 The MobileCoin Foundation
//! SGX Config ID

use crate::new_type_accessors_impls;
use mc_sgx_core_sys_types::{sgx_config_id_t, SGX_CONFIGID_SIZE};
use subtle::{ConstantTimeEq, Choice};

/// Config ID
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[repr(transparent)]
pub struct ConfigId(sgx_config_id_t);

impl ConstantTimeEq for ConfigId {
    fn ct_eq(&self, other: &Self) -> Choice {
        self.ct_eq(&other)
    }
}

new_type_accessors_impls! {
    ConfigId, sgx_config_id_t;
}

// Pure array type larger than 32 so must implement default at the newtype level
impl Default for ConfigId {
    fn default() -> Self {
        Self::from([0; SGX_CONFIGID_SIZE])
    }
}
