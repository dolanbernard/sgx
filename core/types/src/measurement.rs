// Copyright (c) 2018-2023 The MobileCoin Foundation

//! This module contains the wrapper types for an sgx_measurement_t
//!
//! Different types are used for MrSigner and MrEnclave to prevent misuse.

use crate::impl_newtype_for_bytestruct_no_display;
use core::fmt::{Display, Formatter};
use mc_sgx_core_sys_types::{sgx_measurement_t, SGX_HASH_SIZE};

/// An opaque type for MRENCLAVE values
///
/// A MRENCLAVE value is a chained cryptographic hash of the signed
/// enclave binary (.so), and the results of the page initialization
/// steps which created the enclave's pages.
#[derive(Default, Debug, Clone, Eq, PartialEq)]
#[repr(transparent)]
pub struct MrEnclave(sgx_measurement_t);

/// An opaque type for MRSIGNER values.
///
/// A MRSIGNER value is a cryptographic hash of the public key an enclave
/// was signed with.
#[derive(Default, Debug, Clone, Eq, PartialEq)]
#[repr(transparent)]
pub struct MrSigner(sgx_measurement_t);

impl_newtype_for_bytestruct_no_display! {
    MrEnclave, sgx_measurement_t, SGX_HASH_SIZE, m;
    MrSigner, sgx_measurement_t, SGX_HASH_SIZE, m;
}

impl Display for MrEnclave {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "MrEnclave: {:X}", self)
    }
}

impl Display for MrSigner {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "MrSigner: {:X}", self)
    }
}

#[cfg(test)]
mod test {
    extern crate std;

    use super::*;
    use std::format;

    #[test]
    fn from_sgx_mr_enclave() {
        let sgx_mr_enclave = sgx_measurement_t { m: [5u8; 32] };
        let mr_enclave: MrEnclave = sgx_mr_enclave.into();
        assert_eq!(mr_enclave.0, sgx_mr_enclave);
    }

    #[test]
    fn sgx_mr_enclave_from_mr_enclave() {
        let mr_enclave = MrEnclave::default();
        let sgx_mr_enclave: sgx_measurement_t = mr_enclave.into();
        assert_eq!(sgx_mr_enclave.m, [0u8; 32]);
    }

    #[test]
    fn from_sgx_mr_signer() {
        let sgx_mr_signer = sgx_measurement_t { m: [9u8; 32] };
        let mr_signer: MrSigner = sgx_mr_signer.into();
        assert_eq!(mr_signer.0, sgx_mr_signer);
    }

    #[test]
    fn sgx_mr_signer_from_mr_signer() {
        let mr_signer = MrSigner::default();
        let sgx_mr_signer: sgx_measurement_t = mr_signer.into();
        assert_eq!(sgx_mr_signer.m, [0u8; 32]);
    }

    #[test]
    fn display_mr_enclave() {
        let mr_enclave = MrEnclave::from([1u8; MrEnclave::SIZE]);

        let display_string = format!("{}", mr_enclave);
        let expected_string = "MrEnclave: 0101_0101_0101_0101_0101_0101_0101_0101_0101_0101_0101_0101_0101_0101_0101_0101";

        assert_eq!(display_string, expected_string);
    }

    #[test]
    fn display_mr_signer() {
        let mr_signer = MrSigner::from([1u8; MrSigner::SIZE]);

        let display_string = format!("{}", mr_signer);
        let expected_string = "MrSigner: 0101_0101_0101_0101_0101_0101_0101_0101_0101_0101_0101_0101_0101_0101_0101_0101";

        assert_eq!(display_string, expected_string);
    }
}
