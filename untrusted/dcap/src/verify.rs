// Copyright (c) 2022 The MobileCoin Foundation

use crate::{Error, Quote};
use mc_sgx_dcap_sys::{quote3_error_t, sgx_ql_qv_result_t, sgx_qv_verify_quote};
use std::ptr;
use std::time::{SystemTime, UNIX_EPOCH};

/// The sgx_ql_qve_result_t has what are called terminal and non terminal values.
/// They are described in
/// https://download.01.org/intel-sgx/latest/dcap-latest/linux/docs/Intel_SGX_ECDSA_QuoteLibReference_DCAP_API.pdf,
/// but not in the header.  We group them with the QuoteVerificationResult to
/// make it easier to re-use.
enum QuoteVerificationResult {
    Success,
    Terminal(sgx_ql_qv_result_t),
    NonTerminal(sgx_ql_qv_result_t),
}

impl From<sgx_ql_qv_result_t> for QuoteVerificationResult {
    fn from(result: sgx_ql_qv_result_t) -> Self {
        match result {
            sgx_ql_qv_result_t::SGX_QL_QV_RESULT_OK => QuoteVerificationResult::Success,
            sgx_ql_qv_result_t::SGX_QL_QV_RESULT_CONFIG_NEEDED
            | sgx_ql_qv_result_t::SGX_QL_QV_RESULT_OUT_OF_DATE
            | sgx_ql_qv_result_t::SGX_QL_QV_RESULT_OUT_OF_DATE_CONFIG_NEEDED
            | sgx_ql_qv_result_t::SGX_QL_QV_RESULT_SW_HARDENING_NEEDED
            | sgx_ql_qv_result_t::SGX_QL_QV_RESULT_CONFIG_AND_SW_HARDENING_NEEDED => {
                QuoteVerificationResult::NonTerminal(result)
            }
            x => QuoteVerificationResult::Terminal(x),
        }
    }
}

pub trait Verify {
    fn verify(&self) -> Result<(), Error>;
}

impl Verify for Quote {
    fn verify(&self) -> Result<(), Error> {
        let quote = self.quote.as_ptr();
        let quote_length = self.quote.len() as u32;
        let mut expiration_status = 1;
        let mut quote_verification_result = sgx_ql_qv_result_t::SGX_QL_QV_RESULT_UNSPECIFIED;
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Failed computing current time")
            .as_secs()
            .try_into()
            .expect("Couldn't convert u64 seconds to i64");
        let result = unsafe {
            sgx_qv_verify_quote(
                quote,
                quote_length,
                ptr::null(),
                time,
                &mut expiration_status,
                &mut quote_verification_result,
                ptr::null_mut(),
                0,
                ptr::null_mut(),
            )
        };
        Self::map_verify_quote_result(result, quote_verification_result, expiration_status)
    }
}

impl Quote {
    /// Returns the multiple `sgx_qv_verify_quote()` outputs into a consolidated
    /// result.
    ///
    /// There are 3 values returned from `sgx_qv_verify_quote()` that need to be
    /// analyzed in the correct order to determine the final result.
    ///
    /// # Arguments
    /// - `call_result` The return value from the call to `sgx_qv_verify_quote()`.
    /// - `quote_verification_result` the `p_quote_verification_result`
    ///     parameter to `sgx_qv_verify_quote()`.
    /// - `expiration_status` the `p_collateral_expiration_status` parameter
    ///     to `sgx_qv_verify_quote()`.
    fn map_verify_quote_result(
        call_result: quote3_error_t,
        quote_verification_result: sgx_ql_qv_result_t,
        expiration_status: u32,
    ) -> Result<(), Error> {
        match call_result {
            quote3_error_t::SGX_QL_SUCCESS => match quote_verification_result.into() {
                QuoteVerificationResult::Success => match expiration_status {
                    0 => Ok(()),
                    _ => Err(Error::CollateralExpired),
                },
                QuoteVerificationResult::NonTerminal(x) => {
                    Err(Error::NonTerminal(quote3_error_t(x.0)))
                }
                QuoteVerificationResult::Terminal(x) => Err(Error::SgxStatus(quote3_error_t(x.0))),
            },
            x => Err(Error::SgxStatus(x)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    static SW_HARDENING_NEEDED: &[u8] =
        include_bytes!("../../test_enclave/data/sw_hardening_needed_quote.dat");

    #[test]
    fn verify_results_in_unsupported_format_when_empty_quote() {
        // QUOTE_MIN_SIZE is 1020, so just round to a power of 2
        let quote = Quote {
            quote: vec![0; 1024],
        };
        let result = quote.verify();
        assert_eq!(
            result,
            Err(Error::SgxStatus(
                quote3_error_t::SGX_QL_QUOTE_FORMAT_UNSUPPORTED
            ))
        );
    }

    #[test]
    fn verify_results_succeeds_for_good_quote() {
        let quote = Quote {
            quote: SW_HARDENING_NEEDED.to_vec(),
        };
        let result = quote.verify();
        assert_eq!(
            result,
            Err(Error::NonTerminal(quote3_error_t(
                sgx_ql_qv_result_t::SGX_QL_QV_RESULT_SW_HARDENING_NEEDED.0
            )))
        );
    }

    #[rstest]
    #[case(
        quote3_error_t::SGX_QL_ERROR_UNEXPECTED,
        sgx_ql_qv_result_t::SGX_QL_QV_RESULT_OUT_OF_DATE,
        1
    )]
    #[case(
        quote3_error_t::SGX_QL_FILE_ACCESS_ERROR,
        sgx_ql_qv_result_t::SGX_QL_QV_RESULT_OUT_OF_DATE,
        1
    )]
    #[case(
        quote3_error_t::SGX_QL_INVALID_REPORT,
        sgx_ql_qv_result_t::SGX_QL_QV_RESULT_INVALID_SIGNATURE,
        0
    )]
    fn quote_verification_fails_at_call(
        #[case] call_result: quote3_error_t,
        #[case] quote_verification_result: sgx_ql_qv_result_t,
        #[case] expiration_status: u32,
    ) {
        assert_eq!(
            Quote::map_verify_quote_result(
                call_result,
                quote_verification_result,
                expiration_status
            ),
            Err(Error::SgxStatus(call_result))
        );
    }

    #[rstest]
    #[case(
        quote3_error_t::SGX_QL_SUCCESS,
        sgx_ql_qv_result_t::SGX_QL_QV_RESULT_INVALID_SIGNATURE,
        1
    )]
    #[case(
        quote3_error_t::SGX_QL_SUCCESS,
        sgx_ql_qv_result_t::SGX_QL_QV_RESULT_UNSPECIFIED,
        1
    )]
    #[case(
        quote3_error_t::SGX_QL_SUCCESS,
        sgx_ql_qv_result_t::SGX_QL_QV_RESULT_REVOKED,
        0
    )]
    fn quote_verification_result_is_terminal(
        #[case] call_result: quote3_error_t,
        #[case] quote_verification_result: sgx_ql_qv_result_t,
        #[case] expiration_status: u32,
    ) {
        assert_eq!(
            Quote::map_verify_quote_result(
                call_result,
                quote_verification_result,
                expiration_status
            ),
            Err(Error::SgxStatus(quote3_error_t(
                quote_verification_result.0
            )))
        );
    }

    #[rstest]
    #[case(
        quote3_error_t::SGX_QL_SUCCESS,
        sgx_ql_qv_result_t::SGX_QL_QV_RESULT_SW_HARDENING_NEEDED,
        1
    )]
    #[case(
        quote3_error_t::SGX_QL_SUCCESS,
        sgx_ql_qv_result_t::SGX_QL_QV_RESULT_CONFIG_NEEDED,
        0
    )]
    #[case(
        quote3_error_t::SGX_QL_SUCCESS,
        sgx_ql_qv_result_t::SGX_QL_QV_RESULT_CONFIG_AND_SW_HARDENING_NEEDED,
        0
    )]
    #[case(
        quote3_error_t::SGX_QL_SUCCESS,
        sgx_ql_qv_result_t::SGX_QL_QV_RESULT_OUT_OF_DATE,
        0
    )]
    #[case(
        quote3_error_t::SGX_QL_SUCCESS,
        sgx_ql_qv_result_t::SGX_QL_QV_RESULT_OUT_OF_DATE_CONFIG_NEEDED,
        0
    )]
    fn quote_verification_result_is_non_terminal(
        #[case] call_result: quote3_error_t,
        #[case] quote_verification_result: sgx_ql_qv_result_t,
        #[case] expiration_status: u32,
    ) {
        assert_eq!(
            Quote::map_verify_quote_result(
                call_result,
                quote_verification_result,
                expiration_status
            ),
            Err(Error::NonTerminal(quote3_error_t(
                quote_verification_result.0
            )))
        );
    }

    #[rstest]
    #[case(
        quote3_error_t::SGX_QL_SUCCESS,
        sgx_ql_qv_result_t::SGX_QL_QV_RESULT_OK,
        1
    )]
    #[case(
        quote3_error_t::SGX_QL_SUCCESS,
        sgx_ql_qv_result_t::SGX_QL_QV_RESULT_OK,
        3_000_000
    )]
    fn quote_verification_result_is_expired_status(
        #[case] call_result: quote3_error_t,
        #[case] quote_verification_result: sgx_ql_qv_result_t,
        #[case] expiration_status: u32,
    ) {
        assert_eq!(
            Quote::map_verify_quote_result(
                call_result,
                quote_verification_result,
                expiration_status
            ),
            Err(Error::CollateralExpired)
        );
    }

    #[test]
    fn quote_verification_result_is_good() {
        assert_eq!(
            Quote::map_verify_quote_result(
                quote3_error_t::SGX_QL_SUCCESS,
                sgx_ql_qv_result_t::SGX_QL_QV_RESULT_OK,
                0
            ),
            Ok(())
        );
    }
}
